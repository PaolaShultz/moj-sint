use crate::composite_machine::OriginalLayer;
use crate::composite_machine::render_original_layer;
use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::envelope_audition::measure_total_rms;
use crate::hybrid::HybridRenderError;
use crate::hybrid_subset::{
    MAX_CEILING_PROPORTION, MAX_JUMP, MIN_CORRELATION, MIN_TONAL_PASS_FRACTION, OUTPUT_CEILING,
    SubsetCandidate, SubsetMetrics, classify_noise_like, measure_spectral_flatness, measure_subset,
    measure_tonal_pass_fraction,
};
use std::f32::consts::TAU;
use thiserror::Error;

pub const DURATION_MS: u32 = 1_600;
pub const BASE_HZ: f32 = 73.416_19;
pub const MIN_TOTAL_RMS: f64 = 0.199_526_23;
pub const MAX_TOTAL_RMS: f64 = 0.316_227_77;
pub const MAX_ABSOLUTE_DC: f64 = 0.002;

const COUPLED_WIRE_A: [(f32, f32); 8] = [
    (1.0, 1.65),
    (2.003, 1.10),
    (3.012, 0.82),
    (4.028, 0.58),
    (5.055, 0.41),
    (6.09, 0.30),
    (7.13, 0.21),
    (8.18, 0.15),
];
const COUPLED_WIRE_B: [(f32, f32); 8] = [
    (1.0, 1.51),
    (2.003, 0.99),
    (3.012, 0.74),
    (4.028, 0.53),
    (5.055, 0.38),
    (6.09, 0.28),
    (7.13, 0.20),
    (8.18, 0.14),
];
const SPECTRAL_PLATE: [(f32, f32); 11] = [
    (1.0, 1.42),
    (2.01, 0.76),
    (2.74, 0.52),
    (3.89, 0.38),
    (5.13, 0.29),
    (6.47, 0.23),
    (7.92, 0.18),
    (9.51, 0.145),
    (11.2, 0.115),
    (13.4, 0.09),
    (16.1, 0.075),
];
const DUAL_BRIDGE_A: [(f32, f32); 6] = [
    (1.0, 1.60),
    (3.02, 0.77),
    (5.07, 0.49),
    (7.14, 0.33),
    (9.25, 0.23),
    (11.4, 0.16),
];
const DUAL_BRIDGE_B: [(f32, f32); 6] = [
    (2.006, 1.14),
    (4.03, 0.62),
    (6.08, 0.41),
    (8.16, 0.29),
    (10.3, 0.21),
    (12.5, 0.15),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StruckTopology {
    CoupledWire,
    SpectralPlate,
    DualBridge,
}

impl StruckTopology {
    pub const ALL: [Self; 3] = [Self::CoupledWire, Self::SpectralPlate, Self::DualBridge];

    pub const fn exciter(self) -> OriginalLayer {
        match self {
            Self::CoupledWire => OriginalLayer::CrossSingle,
            Self::SpectralPlate => OriginalLayer::SpectralSingle,
            Self::DualBridge => OriginalLayer::DualSingle,
        }
    }

    pub const fn exciter_ms(self) -> u32 {
        match self {
            Self::CoupledWire => 7,
            Self::SpectralPlate => 11,
            Self::DualBridge => 9,
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::CoupledWire => "coupled-wire",
            Self::SpectralPlate => "spectral-plate",
            Self::DualBridge => "dual-bridge",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::CoupledWire => "Coupled Wire",
            Self::SpectralPlate => "Spectral Plate",
            Self::DualBridge => "Dual Bridge",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::CoupledWire => "01_coupled_wire.wav",
            Self::SpectralPlate => "02_spectral_plate.wav",
            Self::DualBridge => "03_dual_bridge.wav",
        }
    }

    pub const fn coupling(self) -> f32 {
        match self {
            Self::CoupledWire => 0.0002,
            Self::SpectralPlate => 0.0,
            Self::DualBridge => 0.0003,
        }
    }

    pub const fn second_bank_delay_ms(self) -> u32 {
        match self {
            Self::CoupledWire => 2,
            Self::SpectralPlate => 0,
            Self::DualBridge => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StruckModeSpec {
    pub bank: usize,
    pub index: usize,
    pub ratio: f32,
    pub decay_seconds: f32,
    pub detune: f32,
}

pub fn mode_specs(topology: StruckTopology) -> Vec<StruckModeSpec> {
    let detuned = 2.0_f32.powf(0.7 / 1_200.0);
    let banks: &[(&[(f32, f32)], f32)] = match topology {
        StruckTopology::CoupledWire => &[(&COUPLED_WIRE_A, 1.0), (&COUPLED_WIRE_B, detuned)],
        StruckTopology::SpectralPlate => &[(&SPECTRAL_PLATE, 1.0)],
        StruckTopology::DualBridge => &[(&DUAL_BRIDGE_A, 1.0), (&DUAL_BRIDGE_B, 1.0)],
    };
    banks
        .iter()
        .enumerate()
        .flat_map(|(bank, (modes, detune))| {
            modes
                .iter()
                .enumerate()
                .map(move |(index, (ratio, decay_seconds))| StruckModeSpec {
                    bank,
                    index,
                    ratio: *ratio,
                    decay_seconds: *decay_seconds,
                    detune: *detune,
                })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum StruckError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error(transparent)]
    Hybrid(#[from] HybridRenderError),
    #[error("gain must be finite and positive")]
    InvalidGain,
    #[error("no shared gain satisfies all presentation limits")]
    NoSharedGain,
}

#[derive(Clone, Copy)]
struct Mode {
    a1: f32,
    a2: f32,
    injection: f32,
    left_weight: f32,
    right_weight: f32,
    y1: f32,
    y2: f32,
}

impl Mode {
    fn new(ratio: f32, decay_seconds: f32, index: usize, sample_rate: u32, detune: f32) -> Self {
        let frequency = BASE_HZ * ratio * detune;
        let radius = (-6.907_755 / (decay_seconds * sample_rate as f32)).exp();
        let angle = TAU * frequency / sample_rate as f32;
        let side = if index % 2 == 0 { 0.08 } else { -0.08 };
        let injection_sign = if index % 2 == 0 { 1.0 } else { -1.0 };
        Self {
            a1: 2.0 * radius * angle.cos(),
            a2: -(radius * radius),
            injection: injection_sign * 0.22 / (1.0 + 0.16 * index as f32),
            left_weight: 1.0 + side,
            right_weight: 1.0 - side,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, force: f32) -> f32 {
        let output = self.injection * force + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = if output.is_finite() { output } else { 0.0 };
        self.y1
    }
}

#[derive(Clone, Copy)]
struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    fn new(sample_rate: u32) -> Self {
        Self {
            coefficient: (-TAU * 35.0 / sample_rate as f32).exp(),
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

pub struct StruckObject {
    topology: StruckTopology,
    banks: Vec<Vec<Mode>>,
    excitation: Vec<f32>,
    frame: usize,
    duration_frames: usize,
    boundary_rise_frames: usize,
    final_fade_frames: usize,
    previous_bank_sums: [f32; 2],
    older_bank_sums: [f32; 2],
    bank_delays: [usize; 2],
    dc_blockers: [DcBlocker; 2],
}

impl StruckObject {
    pub fn new(topology: StruckTopology, sample_rate: u32) -> Result<Self, StruckError> {
        if sample_rate == 0 {
            return Err(StruckError::InvalidSampleRate);
        }
        let excitation = prepare_excitation(topology, sample_rate)?;
        let banks = match topology {
            StruckTopology::CoupledWire => vec![
                make_bank(&COUPLED_WIRE_A, sample_rate, 1.0),
                make_bank(&COUPLED_WIRE_B, sample_rate, 2.0_f32.powf(0.7 / 1_200.0)),
            ],
            StruckTopology::SpectralPlate => {
                vec![make_bank(&SPECTRAL_PLATE, sample_rate, 1.0)]
            }
            StruckTopology::DualBridge => vec![
                make_bank(&DUAL_BRIDGE_A, sample_rate, 1.0),
                make_bank(&DUAL_BRIDGE_B, sample_rate, 1.0),
            ],
        };
        Ok(Self {
            topology,
            banks,
            excitation,
            frame: 0,
            duration_frames: DURATION_MS as usize * sample_rate as usize / 1_000,
            boundary_rise_frames: (sample_rate as usize / 2_000).max(1),
            final_fade_frames: sample_rate as usize / 10,
            previous_bank_sums: [0.0; 2],
            older_bank_sums: [0.0; 2],
            bank_delays: match topology {
                StruckTopology::CoupledWire => [0, 2 * sample_rate as usize / 1_000],
                StruckTopology::DualBridge => [0, 3 * sample_rate as usize / 1_000],
                StruckTopology::SpectralPlate => [0, 0],
            },
            dc_blockers: [DcBlocker::new(sample_rate); 2],
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        if self.frame >= self.duration_frames {
            return StereoFrame::default();
        }
        let coupling = self.topology.coupling();
        let previous = self.previous_bank_sums;
        let older = self.older_bank_sums;
        let mut left = 0.0;
        let mut right = 0.0;
        let mut next_sums = [0.0; 2];
        let bank_counts = [
            self.banks.first().map_or(1, Vec::len),
            self.banks.get(1).map_or(1, Vec::len),
        ];
        for (bank_index, bank) in self.banks.iter_mut().enumerate() {
            let excitation = self
                .frame
                .checked_sub(self.bank_delays[bank_index])
                .and_then(|frame| self.excitation.get(frame))
                .copied()
                .unwrap_or(0.0);
            let other_index = if bank_index == 0 { 1 } else { 0 };
            let other_velocity = previous[other_index] - older[other_index];
            let other_count = if bank_index == 0 {
                bank_counts[1]
            } else {
                bank_counts[0]
            };
            let force = excitation + coupling * other_velocity / other_count as f32;
            for mode in bank.iter_mut() {
                let output = mode.sample(force);
                next_sums[bank_index] += output;
                left += output * mode.left_weight;
                right += output * mode.right_weight;
            }
        }
        self.older_bank_sums = self.previous_bank_sums;
        self.previous_bank_sums = next_sums;
        let normalizer = 1.0 / self.banks.iter().map(Vec::len).sum::<usize>() as f32;
        let boundary = self.boundary_gain(self.frame);
        let left = self.dc_blockers[0].sample(left * normalizer);
        let right = self.dc_blockers[1].sample(right * normalizer);
        self.frame += 1;
        StereoFrame {
            left: left * boundary,
            right: right * boundary,
        }
    }

    #[inline]
    fn boundary_gain(&self, frame: usize) -> f32 {
        if frame < self.boundary_rise_frames {
            return frame as f32 / self.boundary_rise_frames as f32;
        }
        let fade_start = self.duration_frames - self.final_fade_frames;
        if frame >= fade_start {
            return (self.duration_frames - 1 - frame) as f32
                / (self.final_fade_frames - 1).max(1) as f32;
        }
        1.0
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }
}

fn make_bank(modes: &[(f32, f32)], sample_rate: u32, detune: f32) -> Vec<Mode> {
    modes
        .iter()
        .enumerate()
        .map(|(index, &(ratio, decay))| Mode::new(ratio, decay, index, sample_rate, detune))
        .collect()
}

fn prepare_excitation(
    topology: StruckTopology,
    sample_rate: u32,
) -> Result<Vec<f32>, HybridRenderError> {
    let source = render_original_layer(topology.exciter(), sample_rate)?;
    let frames = topology.exciter_ms() as usize * sample_rate as usize / 1_000;
    let force_scale = match topology {
        StruckTopology::CoupledWire => 60.0,
        StruckTopology::SpectralPlate => 8.0,
        StruckTopology::DualBridge => 22.4,
    };
    let mut excitation = Vec::with_capacity(frames);
    let mut previous = 0.0;
    for frame in 0..frames {
        let mono = 0.5 * (source[2 * frame] + source[2 * frame + 1]);
        let differentiated = mono - previous;
        previous = mono;
        let phase = frame as f32 / frames.max(1) as f32;
        let taper = (std::f32::consts::PI * phase).sin().powi(2);
        excitation.push(force_scale * differentiated * taper);
    }
    Ok(excitation)
}

pub fn render_raw(topology: StruckTopology, sample_rate: u32) -> Result<Vec<f32>, StruckError> {
    let mut object = StruckObject::new(topology, sample_rate)?;
    let mut samples = Vec::with_capacity(2 * object.duration_frames());
    for _ in 0..object.duration_frames() {
        let frame = object.sample();
        samples.extend([frame.left, frame.right]);
    }
    Ok(samples)
}

pub fn window_rms(samples: &[f32], start_ms: usize, end_ms: usize, sample_rate: u32) -> f64 {
    let start = 2 * start_ms * sample_rate as usize / 1_000;
    let end = (2 * end_ms * sample_rate as usize / 1_000).min(samples.len());
    let window = &samples[start.min(end)..end];
    if window.is_empty() {
        return 0.0;
    }
    (window.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / window.len() as f64).sqrt()
}

pub fn high_band_ratio(samples: &[f32], start_ms: usize, end_ms: usize, sample_rate: u32) -> f64 {
    let start_frame = start_ms * sample_rate as usize / 1_000;
    let end_frame = (end_ms * sample_rate as usize / 1_000).min(samples.len() / 2);
    let mut total = 0.0;
    let mut difference = 0.0;
    let mut previous = if start_frame == 0 {
        [0.0; 2]
    } else {
        [
            samples[2 * (start_frame - 1)],
            samples[2 * (start_frame - 1) + 1],
        ]
    };
    for frame in start_frame..end_frame {
        for channel in 0..2 {
            let value = samples[2 * frame + channel];
            total += f64::from(value).powi(2);
            difference += f64::from(value - previous[channel]).powi(2);
            previous[channel] = value;
        }
    }
    difference / total.max(1.0e-18)
}

#[derive(Clone, Debug)]
pub struct StruckPreview {
    pub topology: StruckTopology,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct StruckRender {
    pub topology: StruckTopology,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub gain: f32,
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub early_rms: f64,
    pub middle_rms: f64,
    pub late_rms: f64,
    pub early_high_ratio: f64,
    pub middle_high_ratio: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StruckRejection {
    NonFinite,
    TotalRms,
    ExcessiveDc,
    ExcessiveCeiling,
    MaximumJump,
    StereoMono,
    TonalInventory,
    NoiseLike,
    Decay,
    SpectralDecay,
    ReturnToZero,
}

impl StruckRejection {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::NonFinite => "non_finite",
            Self::TotalRms => "total_rms",
            Self::ExcessiveDc => "excessive_dc",
            Self::ExcessiveCeiling => "excessive_ceiling",
            Self::MaximumJump => "maximum_jump",
            Self::StereoMono => "stereo_mono",
            Self::TonalInventory => "tonal_inventory",
            Self::NoiseLike => "noise_like",
            Self::Decay => "decay",
            Self::SpectralDecay => "spectral_decay",
            Self::ReturnToZero => "return_to_zero",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StruckEvidence {
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub early_rms: f64,
    pub middle_rms: f64,
    pub late_rms: f64,
    pub early_high_ratio: f64,
    pub middle_high_ratio: f64,
    pub returns_to_zero: bool,
}

impl StruckEvidence {
    pub fn rejection_reasons(self) -> Vec<StruckRejection> {
        let mut reasons = Vec::new();
        if !self.metrics.finite || !self.total_rms.is_finite() {
            reasons.push(StruckRejection::NonFinite);
        }
        if !(MIN_TOTAL_RMS..=MAX_TOTAL_RMS).contains(&self.total_rms) {
            reasons.push(StruckRejection::TotalRms);
        }
        if self.metrics.dc.abs() > MAX_ABSOLUTE_DC {
            reasons.push(StruckRejection::ExcessiveDc);
        }
        if self.metrics.ceiling_proportion > MAX_CEILING_PROPORTION {
            reasons.push(StruckRejection::ExcessiveCeiling);
        }
        if self.metrics.maximum_jump > MAX_JUMP {
            reasons.push(StruckRejection::MaximumJump);
        }
        if self.metrics.mono_loss_db > 1.0 || self.metrics.correlation <= MIN_CORRELATION {
            reasons.push(StruckRejection::StereoMono);
        }
        if self.tonal_pass_fraction + f64::EPSILON < MIN_TONAL_PASS_FRACTION {
            reasons.push(StruckRejection::TonalInventory);
        }
        if classify_noise_like(self.spectral_flatness, self.tonal_pass_fraction) {
            reasons.push(StruckRejection::NoiseLike);
        }
        let reduction_db =
            20.0 * (self.late_rms.max(1.0e-12) / self.early_rms.max(1.0e-12)).log10();
        if !(self.middle_rms < self.early_rms
            && self.late_rms < self.middle_rms
            && reduction_db <= -30.0)
        {
            reasons.push(StruckRejection::Decay);
        }
        if self.middle_high_ratio >= self.early_high_ratio {
            reasons.push(StruckRejection::SpectralDecay);
        }
        if !self.returns_to_zero {
            reasons.push(StruckRejection::ReturnToZero);
        }
        reasons
    }
}

pub fn preview_all(sample_rate: u32) -> Result<Vec<StruckPreview>, StruckError> {
    StruckTopology::ALL
        .into_iter()
        .map(|topology| preview(topology, sample_rate))
        .collect()
}

pub fn preview(topology: StruckTopology, sample_rate: u32) -> Result<StruckPreview, StruckError> {
    Ok(StruckPreview {
        topology,
        samples: render_raw(topology, sample_rate)?,
        sample_rate,
    })
}

pub fn select_shared_gain(previews: &[StruckPreview]) -> Result<f32, StruckError> {
    if previews.is_empty() {
        return Err(StruckError::NoSharedGain);
    }
    let mut maximum = f32::INFINITY;
    for preview in previews {
        let rms = measure_total_rms(&preview.samples);
        if rms > 0.0 {
            maximum = maximum.min((MAX_TOTAL_RMS / rms) as f32);
        }
        let mut magnitudes = preview.samples.iter().map(|v| v.abs()).collect::<Vec<_>>();
        magnitudes.sort_unstable_by(f32::total_cmp);
        let index = ((magnitudes.len() as f64 * (1.0 - MAX_CEILING_PROPORTION)).ceil() as usize)
            .saturating_sub(1)
            .min(magnitudes.len() - 1);
        if magnitudes[index] > 0.0 {
            maximum = maximum.min(OUTPUT_CEILING / magnitudes[index]);
        }
    }
    let mut gain = (maximum * 100.0).floor() * 0.01;
    while gain > 0.0 {
        if previews.iter().all(|preview| {
            let samples = apply_gain(&preview.samples, gain);
            let metrics = measure_subset(&samples, 0, samples.len() / 2);
            measure_total_rms(&samples) <= MAX_TOTAL_RMS
                && metrics.ceiling_proportion <= MAX_CEILING_PROPORTION
        }) {
            return Ok(gain);
        }
        gain -= 0.01;
    }
    Err(StruckError::NoSharedGain)
}

pub fn render_preview(preview: &StruckPreview, gain: f32) -> Result<StruckRender, StruckError> {
    if !gain.is_finite() || gain <= 0.0 {
        return Err(StruckError::InvalidGain);
    }
    let samples = apply_gain(&preview.samples, gain);
    Ok(StruckRender {
        topology: preview.topology,
        total_rms: measure_total_rms(&samples),
        metrics: measure_subset(&samples, 0, samples.len() / 2),
        early_rms: window_rms(&samples, 50, 200, preview.sample_rate),
        middle_rms: window_rms(&samples, 350, 700, preview.sample_rate),
        late_rms: window_rms(&samples, 1_100, 1_450, preview.sample_rate),
        early_high_ratio: high_band_ratio(&samples, 50, 200, preview.sample_rate),
        middle_high_ratio: high_band_ratio(&samples, 350, 700, preview.sample_rate),
        samples,
        sample_rate: preview.sample_rate,
        gain,
    })
}

pub fn evaluate(render: &StruckRender) -> StruckEvidence {
    StruckEvidence {
        total_rms: render.total_rms,
        metrics: render.metrics,
        tonal_pass_fraction: measure_tonal_pass_fraction(
            SubsetCandidate::ThreeSingles,
            &render.samples,
            render.sample_rate,
        ),
        spectral_flatness: measure_spectral_flatness(&render.samples, 0, render.samples.len() / 2),
        early_rms: render.early_rms,
        middle_rms: render.middle_rms,
        late_rms: render.late_rms,
        early_high_ratio: render.early_high_ratio,
        middle_high_ratio: render.middle_high_ratio,
        returns_to_zero: render.samples[render.samples.len() - 2..]
            .iter()
            .all(|v| v.abs() <= 1.0e-7),
    }
}

fn apply_gain(samples: &[f32], gain: f32) -> Vec<f32> {
    samples
        .iter()
        .map(|value| (value * gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn topologies_are_exact_and_distinct() {
        assert_eq!(
            StruckTopology::ALL,
            [
                StruckTopology::CoupledWire,
                StruckTopology::SpectralPlate,
                StruckTopology::DualBridge,
            ]
        );
        assert_eq!(
            StruckTopology::ALL.map(StruckTopology::exciter),
            [
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(
            StruckTopology::ALL.map(StruckTopology::exciter_ms),
            [7, 11, 9]
        );
        assert_eq!(DURATION_MS, 1_600);
        assert!((BASE_HZ - 73.416_19).abs() < 1.0e-4);
    }

    #[test]
    fn sampling_is_finite_distinct_and_allocation_free() {
        let mut hashes = Vec::new();
        for topology in StruckTopology::ALL {
            let mut object = StruckObject::new(topology, 8_000).unwrap();
            assert_eq!(object.duration_frames(), 12_800);
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            let mut nonzero = false;
            for _ in 0..object.duration_frames() {
                let frame = assert_no_alloc(|| object.sample());
                assert!(frame.left.is_finite() && frame.right.is_finite());
                nonzero |= frame.left != 0.0 || frame.right != 0.0;
                for value in [frame.left, frame.right] {
                    for byte in value.to_bits().to_le_bytes() {
                        hash ^= u64::from(byte);
                        hash = hash.wrapping_mul(0x100_0000_01b3);
                    }
                }
            }
            assert!(nonzero);
            assert_eq!(assert_no_alloc(|| object.sample()), StereoFrame::default());
            hashes.push(hash);
        }
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), 3);
    }

    #[test]
    fn modal_energy_and_high_band_decay_without_a_sustain_plateau() {
        for topology in StruckTopology::ALL {
            let samples = render_raw(topology, 48_000).unwrap();
            let early = window_rms(&samples, 50, 200, 48_000);
            let middle = window_rms(&samples, 350, 700, 48_000);
            let late = window_rms(&samples, 1_100, 1_450, 48_000);
            assert!(middle < early, "{} {early} {middle}", topology.slug());
            assert!(late < middle, "{} {middle} {late}", topology.slug());
            assert!(
                20.0 * (late.max(1.0e-12) / early.max(1.0e-12)).log10() <= -30.0,
                "{} {early} {late}",
                topology.slug()
            );
            let early_high = high_band_ratio(&samples, 50, 200, 48_000);
            let middle_high = high_band_ratio(&samples, 350, 700, 48_000);
            assert!(
                middle_high < early_high,
                "{} {early_high} {middle_high}",
                topology.slug()
            );
        }
    }

    #[test]
    fn shared_gain_keeps_all_objects_inside_the_engineering_gate() {
        let previews = preview_all(48_000).unwrap();
        let gain = select_shared_gain(&previews).unwrap();
        for preview in &previews {
            let render = render_preview(preview, gain).unwrap();
            assert!(
                (MIN_TOTAL_RMS..=MAX_TOTAL_RMS).contains(&render.total_rms),
                "{} {} {gain}",
                render.topology.slug(),
                render.total_rms
            );
            let reasons = evaluate(&render).rejection_reasons();
            assert!(
                reasons.is_empty(),
                "{} {:?}",
                render.topology.slug(),
                evaluate(&render)
            );
        }
    }

    #[test]
    fn weak_object_is_rejected_by_whole_file_rms() {
        let preview = preview(StruckTopology::CoupledWire, 8_000).unwrap();
        let render = render_preview(&preview, 0.01).unwrap();
        assert!(
            evaluate(&render)
                .rejection_reasons()
                .contains(&StruckRejection::TotalRms)
        );
    }
}
