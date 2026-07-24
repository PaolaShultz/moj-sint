use crate::composite_machine::OriginalLayer;
use crate::composite_machine::render_original_layer;
use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::hybrid::HybridRenderError;
use std::f32::consts::TAU;
use thiserror::Error;

pub const DURATION_MS: u32 = 1_600;
pub const BASE_HZ: f32 = 73.416_19;

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
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum StruckError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error(transparent)]
    Hybrid(#[from] HybridRenderError),
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
        Self {
            a1: 2.0 * radius * angle.cos(),
            a2: -(radius * radius),
            injection: 0.22 / (1.0 + 0.16 * index as f32),
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
}

impl StruckObject {
    pub fn new(topology: StruckTopology, sample_rate: u32) -> Result<Self, StruckError> {
        if sample_rate == 0 {
            return Err(StruckError::InvalidSampleRate);
        }
        let excitation = prepare_excitation(topology, sample_rate)?;
        let banks = match topology {
            StruckTopology::CoupledWire => vec![
                make_bank(
                    &[1.0, 2.003, 3.012, 4.028, 5.055, 6.09, 7.13, 8.18],
                    &[1.30, 1.02, 0.78, 0.56, 0.40, 0.29, 0.21, 0.15],
                    sample_rate,
                    1.0,
                ),
                make_bank(
                    &[1.0, 2.003, 3.012, 4.028, 5.055, 6.09, 7.13, 8.18],
                    &[1.18, 0.92, 0.70, 0.51, 0.37, 0.27, 0.19, 0.14],
                    sample_rate,
                    2.0_f32.powf(0.7 / 1_200.0),
                ),
            ],
            StruckTopology::SpectralPlate => vec![make_bank(
                &[
                    1.0, 2.01, 2.74, 3.89, 5.13, 6.47, 7.92, 9.51, 11.2, 13.4, 16.1,
                ],
                &[
                    1.10, 0.76, 0.52, 0.38, 0.29, 0.23, 0.18, 0.145, 0.115, 0.09, 0.075,
                ],
                sample_rate,
                1.0,
            )],
            StruckTopology::DualBridge => vec![
                make_bank(
                    &[1.0, 3.02, 5.07, 7.14, 9.25, 11.4],
                    &[1.25, 0.72, 0.46, 0.31, 0.22, 0.16],
                    sample_rate,
                    1.0,
                ),
                make_bank(
                    &[2.006, 4.03, 6.08, 8.16, 10.3, 12.5],
                    &[0.90, 0.58, 0.39, 0.28, 0.20, 0.15],
                    sample_rate,
                    1.0,
                ),
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
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        if self.frame >= self.duration_frames {
            return StereoFrame::default();
        }
        let excitation = self.excitation.get(self.frame).copied().unwrap_or(0.0);
        let coupling = match self.topology {
            StruckTopology::CoupledWire => 0.0002,
            StruckTopology::DualBridge => 0.0003,
            StruckTopology::SpectralPlate => 0.0,
        };
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
        self.frame += 1;
        StereoFrame {
            left: left * normalizer * boundary,
            right: right * normalizer * boundary,
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

fn make_bank(ratios: &[f32], decays: &[f32], sample_rate: u32, detune: f32) -> Vec<Mode> {
    ratios
        .iter()
        .zip(decays)
        .enumerate()
        .map(|(index, (&ratio, &decay))| Mode::new(ratio, decay, index, sample_rate, detune))
        .collect()
}

fn prepare_excitation(
    topology: StruckTopology,
    sample_rate: u32,
) -> Result<Vec<f32>, HybridRenderError> {
    let source = render_original_layer(topology.exciter(), sample_rate)?;
    let frames = topology.exciter_ms() as usize * sample_rate as usize / 1_000;
    let mut excitation = Vec::with_capacity(frames);
    let mut previous = 0.0;
    for frame in 0..frames {
        let mono = 0.5 * (source[2 * frame] + source[2 * frame + 1]);
        let differentiated = mono - previous;
        previous = mono;
        let phase = frame as f32 / frames.max(1) as f32;
        let taper = (std::f32::consts::PI * phase).sin().powi(2);
        excitation.push(8.0 * differentiated * taper);
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
    let mut previous = [0.0_f32; 2];
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
}
