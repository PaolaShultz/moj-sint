use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::envelope_audition::measure_total_rms;
use crate::hybrid_subset::{
    MAX_JUMP, MIN_CORRELATION, MIN_TONAL_PASS_FRACTION, OUTPUT_CEILING, SubsetCandidate,
    classify_noise_like, measure_spectral_flatness, measure_subset, measure_tonal_pass_fraction,
};
use crate::struck_object::{MAX_ABSOLUTE_DC, StruckError, StruckObject, StruckTopology};
use std::f32::consts::TAU;

pub const LOW_SPLIT_HZ: f32 = 105.0;
pub const THUMP_SPLIT_HZ: f32 = 500.0;
pub const LOW_STRIKE_GAIN: f32 = 0.794_328_2;
pub const LOW_CREST_GAIN: f32 = 0.55;
pub const LOW_CREST_END_MS: u32 = 25;
pub const LOW_UNITY_MS: u32 = 50;
pub const WET_START_MS: u32 = 8;
pub const WET_FULL_MS: u32 = 12;
pub const WET_HOLD_END_MS: u32 = 45;
pub const WET_END_MS: u32 = 70;
pub const LOW_HOLD_END_MS: u32 = 80;
pub const LOW_RECOVERY_END_MS: u32 = 130;
pub const UPPER_STRIKE_GAIN: f32 = 0.02;
pub const PRESENTATION_GAINS: [f32; 9] = [5.00, 4.90, 4.80, 4.70, 4.60, 4.50, 4.40, 4.30, 4.20];

const SAMPLE_PEAK: f32 = 0.794_328_2;
const TRUE_PEAK: f64 = 0.841_395_1;
const REFERENCE_GAIN: f32 = 4.43;
const REFERENCE_LOW_ONSET_DBFS: f64 = -7.455_548;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumpConfig {
    pub slug: &'static str,
    pub drive: f32,
    pub threshold: f32,
    pub wet: f32,
}

pub const CONFIGS: [ThumpConfig; 3] = [
    ThumpConfig {
        slug: "gentle",
        drive: 1.5,
        threshold: 0.20,
        wet: 0.24,
    },
    ThumpConfig {
        slug: "controlled",
        drive: 1.8,
        threshold: 0.15,
        wet: 0.30,
    },
    ThumpConfig {
        slug: "hard",
        drive: 2.1,
        threshold: 0.14,
        wet: 0.35,
    },
];

#[derive(Clone, Copy, Debug, Default)]
pub struct ThumpFrame {
    pub source: StereoFrame,
    pub output: StereoFrame,
    pub clean_low: f32,
    pub nonlinear_residual: StereoFrame,
}

#[derive(Clone, Copy)]
struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(hz: f32, sample_rate: u32) -> Self {
        Self {
            coefficient: (-TAU * hz / sample_rate as f32).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.state = (1.0 - self.coefficient) * input + self.coefficient * self.state;
        self.state
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

#[derive(Clone, Copy, Default)]
struct HardCrestAdaa {
    previous_input: f32,
    previous_driven: f32,
}

impl HardCrestAdaa {
    #[inline]
    fn sample_generated(&mut self, input: f32, config: ThumpConfig) -> f32 {
        let driven = input * config.drive;
        let difference = driven - self.previous_driven;
        let clipped = if difference.abs() > 1.0e-6 {
            (hard_crest_antiderivative(driven, config.threshold)
                - hard_crest_antiderivative(self.previous_driven, config.threshold))
                / difference
        } else {
            crest_clip(0.5 * (driven + self.previous_driven), config.threshold)
        };
        let clean = 0.5 * (input + self.previous_input);
        self.previous_input = input;
        self.previous_driven = driven;
        clipped - clean
    }
}

#[derive(Clone, Copy)]
struct BiquadHighPass {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadHighPass {
    fn new(hz: f32, sample_rate: u32) -> Self {
        let k = (std::f32::consts::PI * hz / sample_rate as f32).tan();
        let norm = 1.0 / (1.0 + std::f32::consts::SQRT_2 * k + k * k);
        Self {
            b0: norm,
            b1: -2.0 * norm,
            b2: norm,
            a1: 2.0 * (k * k - 1.0) * norm,
            a2: (1.0 - std::f32::consts::SQRT_2 * k + k * k) * norm,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }
}

fn make_residual_highpass(sample_rate: u32) -> [BiquadHighPass; 6] {
    [
        BiquadHighPass::new(LOW_SPLIT_HZ, sample_rate),
        BiquadHighPass::new(LOW_SPLIT_HZ, sample_rate),
        BiquadHighPass::new(LOW_SPLIT_HZ, sample_rate),
        BiquadHighPass::new(LOW_SPLIT_HZ, sample_rate),
        BiquadHighPass::new(LOW_SPLIT_HZ, sample_rate),
        BiquadHighPass::new(45.0, sample_rate),
    ]
}

pub struct ControlledThumpVoice {
    body: StruckObject,
    config: ThumpConfig,
    sample_rate: u32,
    frame: usize,
    duration_frames: usize,
    low_split: OnePole,
    thump_split: [OnePole; 2],
    shapers: [HardCrestAdaa; 2],
    residual_highpass: [[BiquadHighPass; 6]; 2],
    post_dc: [DcBlocker; 2],
}

#[derive(Clone, Debug)]
pub struct ThumpPreview {
    pub config: ThumpConfig,
    pub source: Vec<f32>,
    pub samples: Vec<f32>,
    pub clean_low: Vec<f32>,
    pub nonlinear_residual: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct ThumpRender {
    pub config: ThumpConfig,
    pub gain: f32,
    pub source: Vec<f32>,
    pub samples: Vec<f32>,
    pub clean_low: Vec<f32>,
    pub nonlinear_residual: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ThumpEvidence {
    pub total_rms_dbfs: f64,
    pub sample_peak_dbfs: f64,
    pub true_peak_dbfs: f64,
    pub low_onset_reduction_db: f64,
    pub low_nonlinear_residual_db: f64,
    pub thump_residual_db: f64,
    pub high_rate_residual_db: f64,
    pub reference_high_rate_residual_db: f64,
    pub metrics: crate::hybrid_subset::SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub returns_to_zero: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThumpRejection {
    NonFinite,
    WeakOrExcessiveRms,
    SamplePeak,
    TruePeak,
    CeilingContact,
    LowOnset,
    LowNonlinearLeakage,
    ThumpResidual,
    DcOrJump,
    TonalInventory,
    NoiseLike,
    StereoMono,
    Aliasing,
    ReturnToZero,
}

impl ThumpEvidence {
    pub fn rejection_reasons(self) -> Vec<ThumpRejection> {
        let mut reasons = Vec::new();
        if !self.metrics.finite
            || !self.total_rms_dbfs.is_finite()
            || !self.sample_peak_dbfs.is_finite()
            || !self.true_peak_dbfs.is_finite()
        {
            reasons.push(ThumpRejection::NonFinite);
        }
        if !(-15.5..=-11.5).contains(&self.total_rms_dbfs) {
            reasons.push(ThumpRejection::WeakOrExcessiveRms);
        }
        if self.metrics.peak > f64::from(SAMPLE_PEAK) + 1.0e-7 {
            reasons.push(ThumpRejection::SamplePeak);
        }
        if 10.0_f64.powf(self.true_peak_dbfs / 20.0) > TRUE_PEAK + 1.0e-7 {
            reasons.push(ThumpRejection::TruePeak);
        }
        if self.metrics.ceiling_proportion > 0.0 {
            reasons.push(ThumpRejection::CeilingContact);
        }
        if !(-4.0..=-1.5).contains(&self.low_onset_reduction_db) {
            reasons.push(ThumpRejection::LowOnset);
        }
        if self.low_nonlinear_residual_db > -40.0 {
            reasons.push(ThumpRejection::LowNonlinearLeakage);
        }
        if !(-30.0..=-12.0).contains(&self.thump_residual_db) {
            reasons.push(ThumpRejection::ThumpResidual);
        }
        if self.metrics.dc.abs() > MAX_ABSOLUTE_DC || self.metrics.maximum_jump > MAX_JUMP {
            reasons.push(ThumpRejection::DcOrJump);
        }
        if self.tonal_pass_fraction + f64::EPSILON < MIN_TONAL_PASS_FRACTION {
            reasons.push(ThumpRejection::TonalInventory);
        }
        if classify_noise_like(self.spectral_flatness, self.tonal_pass_fraction) {
            reasons.push(ThumpRejection::NoiseLike);
        }
        if self.metrics.mono_loss_db > 1.0 || self.metrics.correlation <= MIN_CORRELATION {
            reasons.push(ThumpRejection::StereoMono);
        }
        if self.high_rate_residual_db > -50.0 {
            reasons.push(ThumpRejection::Aliasing);
        }
        if !self.returns_to_zero {
            reasons.push(ThumpRejection::ReturnToZero);
        }
        reasons
    }
}

pub fn preview(config: ThumpConfig, sample_rate: u32) -> Result<ThumpPreview, StruckError> {
    let mut voice = ControlledThumpVoice::new(config, sample_rate)?;
    let mut source = Vec::with_capacity(2 * voice.duration_frames());
    let mut samples = Vec::with_capacity(2 * voice.duration_frames());
    let mut clean_low = Vec::with_capacity(2 * voice.duration_frames());
    let mut nonlinear_residual = Vec::with_capacity(2 * voice.duration_frames());
    for _ in 0..voice.duration_frames() {
        let trace = voice.sample_trace();
        source.extend([trace.source.left, trace.source.right]);
        samples.extend([trace.output.left, trace.output.right]);
        clean_low.extend([trace.clean_low, trace.clean_low]);
        nonlinear_residual.extend([
            trace.nonlinear_residual.left,
            trace.nonlinear_residual.right,
        ]);
    }
    Ok(ThumpPreview {
        config,
        source,
        samples,
        clean_low,
        nonlinear_residual,
        sample_rate,
    })
}

pub fn render(preview: &ThumpPreview, gain: f32) -> Result<ThumpRender, StruckError> {
    if !gain.is_finite() || gain <= 0.0 {
        return Err(StruckError::InvalidGain);
    }
    Ok(ThumpRender {
        config: preview.config,
        gain,
        source: preview.source.iter().map(|value| value * gain).collect(),
        samples: preview.samples.iter().map(|value| value * gain).collect(),
        clean_low: preview.clean_low.iter().map(|value| value * gain).collect(),
        nonlinear_residual: preview
            .nonlinear_residual
            .iter()
            .map(|value| value * gain)
            .collect(),
        sample_rate: preview.sample_rate,
    })
}

pub fn evaluate(render: &ThumpRender) -> ThumpEvidence {
    let high_rate_residual_db =
        measure_high_rate_residual(render.config, render.sample_rate).unwrap_or(f64::INFINITY);
    let reference_high_rate_residual_db =
        measure_rejected_reference_high_rate_residual(render.sample_rate)
            .unwrap_or(f64::NEG_INFINITY);
    evaluate_with_alias(
        render,
        high_rate_residual_db,
        reference_high_rate_residual_db,
    )
}

pub fn select_candidate(sample_rate: u32) -> Result<ThumpRender, StruckError> {
    let reference_alias = measure_rejected_reference_high_rate_residual(sample_rate)?;
    for config in CONFIGS {
        let candidate_alias = measure_high_rate_residual(config, sample_rate)?;
        let preview = preview(config, sample_rate)?;
        for gain in PRESENTATION_GAINS {
            let render = render(&preview, gain)?;
            let evidence = evaluate_with_alias(&render, candidate_alias, reference_alias);
            if evidence.rejection_reasons().is_empty() {
                return Ok(render);
            }
        }
    }
    Err(StruckError::NoSharedGain)
}

fn evaluate_with_alias(
    render: &ThumpRender,
    high_rate_residual_db: f64,
    reference_high_rate_residual_db: f64,
) -> ThumpEvidence {
    let total_rms = measure_total_rms(&render.samples);
    let peak = render
        .samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f32, f32::max);
    let metrics = measure_subset(&render.samples, 0, render.samples.len() / 2);
    let low_onset_dbfs = linear_db(band_rms(
        &render.samples,
        render.sample_rate,
        0,
        100,
        45.0,
        90.0,
    ));
    let low_residual = band_rms(
        &render.nonlinear_residual,
        render.sample_rate,
        0,
        100,
        0.0,
        LOW_SPLIT_HZ,
    );
    let clean_low = band_rms(
        &render.clean_low,
        render.sample_rate,
        0,
        100,
        0.0,
        LOW_SPLIT_HZ,
    );
    let thump_residual = band_rms(
        &render.nonlinear_residual,
        render.sample_rate,
        WET_START_MS as usize,
        WET_END_MS as usize,
        LOW_SPLIT_HZ,
        THUMP_SPLIT_HZ,
    );
    let clean_thump = band_rms(
        &render.source,
        render.sample_rate,
        WET_START_MS as usize,
        WET_END_MS as usize,
        LOW_SPLIT_HZ,
        THUMP_SPLIT_HZ,
    );
    ThumpEvidence {
        total_rms_dbfs: linear_db(total_rms),
        sample_peak_dbfs: linear_db(f64::from(peak)),
        true_peak_dbfs: linear_db(measure_true_peak(&render.samples)),
        low_onset_reduction_db: low_onset_dbfs - REFERENCE_LOW_ONSET_DBFS,
        low_nonlinear_residual_db: ratio_db(low_residual, clean_low),
        thump_residual_db: ratio_db(thump_residual, clean_thump),
        high_rate_residual_db,
        reference_high_rate_residual_db,
        metrics,
        tonal_pass_fraction: measure_tonal_pass_fraction(
            SubsetCandidate::ThreeSingles,
            &render.samples,
            render.sample_rate,
        ),
        spectral_flatness: measure_spectral_flatness(&render.samples, 0, render.samples.len() / 2),
        returns_to_zero: render.samples[render.samples.len() - 2..]
            .iter()
            .all(|value| value.abs() <= 1.0e-7),
    }
}

pub fn measure_high_rate_residual(
    config: ThumpConfig,
    sample_rate: u32,
) -> Result<f64, StruckError> {
    measure_rate_residual(config, sample_rate, false)
}

pub fn measure_rejected_reference_high_rate_residual(sample_rate: u32) -> Result<f64, StruckError> {
    measure_rate_residual(CONFIGS[0], sample_rate, true)
}

fn measure_rate_residual(
    config: ThumpConfig,
    sample_rate: u32,
    rejected_reference: bool,
) -> Result<f64, StruckError> {
    const FACTOR: usize = 8;
    let frames = sample_rate as usize / 10;
    let target = render_alias_probe(config, sample_rate, frames, rejected_reference);
    let high_rate = render_alias_probe(
        config,
        sample_rate * FACTOR as u32,
        frames * FACTOR,
        rejected_reference,
    );
    let normalization = (0..frames)
        .map(|frame| alias_probe_input(frame, sample_rate, rejected_reference))
        .collect::<Vec<_>>();
    const EDGE: usize = 5;
    let mut best = f64::INFINITY;
    for center_offset in -8..=8 {
        let reference = downsample_8x(&high_rate, frames, center_offset);
        best = best.min(fitted_residual_db(
            &target[EDGE..frames - EDGE],
            &reference[EDGE..frames - EDGE],
            &normalization[EDGE..frames - EDGE],
        ));
    }
    Ok(best)
}

fn downsample_8x(high_rate: &[f32], output_frames: usize, center_offset: isize) -> Vec<f32> {
    const FACTOR: isize = 8;
    const RADIUS: isize = 32;
    (0..output_frames)
        .map(|frame| {
            let center = frame as isize * FACTOR + center_offset;
            let mut output = 0.0;
            let mut weight = 0.0;
            for offset in -RADIUS..=RADIUS {
                let index = center + offset;
                if !(0..high_rate.len() as isize).contains(&index) {
                    continue;
                }
                let distance = offset as f64 / FACTOR as f64;
                let sinc = if offset == 0 {
                    1.0
                } else {
                    (std::f64::consts::PI * distance).sin() / (std::f64::consts::PI * distance)
                };
                let window =
                    0.5 + 0.5 * (std::f64::consts::PI * offset as f64 / RADIUS as f64).cos();
                let coefficient = sinc * window;
                output += f64::from(high_rate[index as usize]) * coefficient;
                weight += coefficient;
            }
            (output / weight.max(1.0e-18)) as f32
        })
        .collect()
}

fn render_alias_probe(
    config: ThumpConfig,
    sample_rate: u32,
    frames: usize,
    rejected_reference: bool,
) -> Vec<f32> {
    let mut shaper = HardCrestAdaa::default();
    let mut highpass = make_residual_highpass(sample_rate);
    let mut dc = DcBlocker::new(sample_rate);
    (0..frames)
        .map(|frame| {
            let input = alias_probe_input(frame, sample_rate, rejected_reference);
            if rejected_reference {
                let linear = input * REFERENCE_GAIN;
                linear.clamp(-OUTPUT_CEILING, OUTPUT_CEILING) - linear
            } else {
                let mut residual = wet_envelope_at(frame, sample_rate)
                    * config.wet
                    * shaper.sample_generated(input, config);
                for stage in &mut highpass {
                    residual = stage.sample(residual);
                }
                dc.sample(residual)
            }
        })
        .collect()
}

fn alias_probe_input(frame: usize, sample_rate: u32, rejected_reference: bool) -> f32 {
    let time = frame as f32 / sample_rate as f32;
    let mut input = 0.28 * (TAU * 146.832_38 * time).sin() + 0.14 * (TAU * 293.664_76 * time).sin();
    if rejected_reference {
        input += 0.08 * (TAU * 600.0 * time).sin();
    }
    input
}

fn fitted_residual_db(target: &[f32], reference: &[f32], normalization: &[f32]) -> f64 {
    let target_mean =
        target.iter().map(|value| f64::from(*value)).sum::<f64>() / target.len() as f64;
    let reference_mean =
        reference.iter().map(|value| f64::from(*value)).sum::<f64>() / reference.len() as f64;
    let mut dot = 0.0;
    let mut reference_energy = 0.0;
    let normalization_mean = normalization
        .iter()
        .map(|value| f64::from(*value))
        .sum::<f64>()
        / normalization.len() as f64;
    for (&target, &reference) in target.iter().zip(reference) {
        let target = f64::from(target) - target_mean;
        let reference = f64::from(reference) - reference_mean;
        dot += target * reference;
        reference_energy += reference * reference;
    }
    let gain = dot / reference_energy.max(1.0e-24);
    let residual_energy = target
        .iter()
        .zip(reference)
        .map(|(&target, &reference)| {
            let target = f64::from(target) - target_mean;
            let reference = f64::from(reference) - reference_mean;
            (target - gain * reference).powi(2)
        })
        .sum::<f64>();
    let normalization_energy = normalization
        .iter()
        .map(|value| (f64::from(*value) - normalization_mean).powi(2))
        .sum::<f64>();
    10.0 * (residual_energy / normalization_energy.max(1.0e-24))
        .max(1.0e-24)
        .log10()
}

fn band_rms(
    samples: &[f32],
    sample_rate: u32,
    start_ms: usize,
    end_ms: usize,
    low_hz: f32,
    high_hz: f32,
) -> f64 {
    let mut low = [OnePole::new(low_hz.max(1.0), sample_rate); 2];
    let mut high = [OnePole::new(high_hz, sample_rate); 2];
    let start = start_ms * sample_rate as usize / 1_000;
    let end = (end_ms * sample_rate as usize / 1_000).min(samples.len() / 2);
    let mut energy = 0.0;
    let mut count = 0;
    for frame in 0..end {
        for channel in 0..2 {
            let input = samples[2 * frame + channel];
            let above_low = if low_hz > 0.0 {
                input - low[channel].sample(input)
            } else {
                input
            };
            let band = high[channel].sample(above_low);
            if frame >= start {
                energy += f64::from(band).powi(2);
                count += 1;
            }
        }
    }
    (energy / count.max(1) as f64).sqrt()
}

fn measure_true_peak(samples: &[f32]) -> f64 {
    const PHASES: usize = 8;
    const RADIUS: isize = 16;
    let frames = samples.len() / 2;
    let mut peak = samples
        .iter()
        .map(|value| f64::from(value.abs()))
        .fold(0.0_f64, f64::max);
    for channel in 0..2 {
        for frame in 0..frames.saturating_sub(1) {
            for phase in 1..PHASES {
                let position = frame as f64 + phase as f64 / PHASES as f64;
                let mut value = 0.0;
                let mut weight = 0.0;
                for tap in -RADIUS + 1..=RADIUS {
                    let index = frame as isize + tap;
                    if !(0..frames as isize).contains(&index) {
                        continue;
                    }
                    let distance = position - index as f64;
                    let sinc = if distance.abs() <= f64::EPSILON {
                        1.0
                    } else {
                        (std::f64::consts::PI * distance).sin() / (std::f64::consts::PI * distance)
                    };
                    let window_position = distance / RADIUS as f64;
                    let window = 0.5 + 0.5 * (std::f64::consts::PI * window_position).cos();
                    let coefficient = sinc * window;
                    value += f64::from(samples[2 * index as usize + channel]) * coefficient;
                    weight += coefficient;
                }
                peak = peak.max((value / weight.max(1.0e-18)).abs());
            }
        }
    }
    peak
}

fn ratio_db(numerator: f64, denominator: f64) -> f64 {
    20.0 * (numerator.max(1.0e-12) / denominator.max(1.0e-12)).log10()
}

fn linear_db(value: f64) -> f64 {
    20.0 * value.max(1.0e-12).log10()
}

impl ControlledThumpVoice {
    pub fn new(config: ThumpConfig, sample_rate: u32) -> Result<Self, StruckError> {
        if sample_rate == 0
            || !config.drive.is_finite()
            || config.drive <= 0.0
            || !config.threshold.is_finite()
            || config.threshold <= 0.0
            || !config.wet.is_finite()
            || !(0.0..=0.35).contains(&config.wet)
        {
            return Err(StruckError::InvalidConfiguration);
        }
        let body = StruckObject::new(StruckTopology::CoupledWire, sample_rate)?;
        let duration_frames = body.duration_frames();
        Ok(Self {
            body,
            config,
            sample_rate,
            frame: 0,
            duration_frames,
            low_split: OnePole::new(LOW_SPLIT_HZ, sample_rate),
            thump_split: [OnePole::new(THUMP_SPLIT_HZ, sample_rate); 2],
            shapers: [HardCrestAdaa::default(); 2],
            residual_highpass: [make_residual_highpass(sample_rate); 2],
            post_dc: [DcBlocker::new(sample_rate); 2],
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        self.sample_trace().output
    }

    #[inline]
    pub fn sample_trace(&mut self) -> ThumpFrame {
        if self.frame >= self.duration_frames {
            return ThumpFrame::default();
        }
        let source = self.body.sample();
        let mid = 0.5 * (source.left + source.right);
        let clean_low = self.low_split.sample(mid);
        let upper_left = source.left - clean_low;
        let upper_right = source.right - clean_low;
        let thump_left = self.thump_split[0].sample(upper_left);
        let thump_right = self.thump_split[1].sample(upper_right);
        let bright_left = upper_left - thump_left;
        let bright_right = upper_right - thump_right;

        let wet = self.wet_envelope();
        let generated_left = self.shapers[0].sample_generated(thump_left, self.config);
        let generated_right = self.shapers[1].sample_generated(thump_right, self.config);
        let (residual_left, residual_right) = if wet > 0.0 {
            let raw_left = wet * self.config.wet * generated_left;
            let raw_right = wet * self.config.wet * generated_right;
            let mut left = raw_left;
            let mut right = raw_right;
            for stage in &mut self.residual_highpass[0] {
                left = stage.sample(left);
            }
            for stage in &mut self.residual_highpass[1] {
                right = stage.sample(right);
            }
            (self.post_dc[0].sample(left), self.post_dc[1].sample(right))
        } else {
            (0.0, 0.0)
        };
        let low = clean_low * self.low_contour();
        let upper = self.upper_contour();
        let output = StereoFrame {
            left: low + upper * (bright_left + thump_left) + residual_left,
            right: low + upper * (bright_right + thump_right) + residual_right,
        };
        self.frame += 1;
        ThumpFrame {
            source,
            output,
            clean_low,
            nonlinear_residual: StereoFrame {
                left: residual_left,
                right: residual_right,
            },
        }
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }

    #[inline]
    fn wet_envelope(&self) -> f32 {
        wet_envelope_at(self.frame, self.sample_rate)
    }

    #[inline]
    fn low_contour(&self) -> f32 {
        let milliseconds = self.frame as f32 * 1_000.0 / self.sample_rate as f32;
        if milliseconds < WET_START_MS as f32 {
            LOW_STRIKE_GAIN
        } else if milliseconds < WET_FULL_MS as f32 {
            let phase = (milliseconds - WET_START_MS as f32) / (WET_FULL_MS - WET_START_MS) as f32;
            LOW_STRIKE_GAIN + (LOW_CREST_GAIN - LOW_STRIKE_GAIN) * smoothstep(phase)
        } else if milliseconds < LOW_CREST_END_MS as f32 {
            LOW_CREST_GAIN
        } else if milliseconds < LOW_UNITY_MS as f32 {
            let phase =
                (milliseconds - LOW_CREST_END_MS as f32) / (LOW_UNITY_MS - LOW_CREST_END_MS) as f32;
            LOW_CREST_GAIN + (1.0 - LOW_CREST_GAIN) * smoothstep(phase)
        } else {
            1.0
        }
    }

    #[inline]
    fn upper_contour(&self) -> f32 {
        let milliseconds = self.frame as f32 * 1_000.0 / self.sample_rate as f32;
        if milliseconds < LOW_HOLD_END_MS as f32 {
            UPPER_STRIKE_GAIN
        } else if milliseconds < LOW_RECOVERY_END_MS as f32 {
            let phase = (milliseconds - LOW_HOLD_END_MS as f32)
                / (LOW_RECOVERY_END_MS - LOW_HOLD_END_MS) as f32;
            UPPER_STRIKE_GAIN + (1.0 - UPPER_STRIKE_GAIN) * smoothstep(phase)
        } else {
            1.0
        }
    }
}

#[inline]
fn crest_clip(input: f32, threshold: f32) -> f32 {
    let normalized = input / threshold;
    if normalized > 1.0 {
        (2.0 / 3.0) * threshold
    } else if normalized < -1.0 {
        -(2.0 / 3.0) * threshold
    } else {
        threshold * (normalized - normalized * normalized * normalized / 3.0)
    }
}

#[inline]
fn hard_crest_antiderivative(input: f32, threshold: f32) -> f32 {
    let normalized = input / threshold;
    let integral = if normalized.abs() <= 1.0 {
        let squared = normalized * normalized;
        0.5 * squared - squared * squared / 12.0
    } else {
        (2.0 / 3.0) * normalized.abs() - 0.25
    };
    threshold * threshold * integral
}

#[inline]
fn wet_envelope_at(frame: usize, sample_rate: u32) -> f32 {
    let milliseconds = frame as f32 * 1_000.0 / sample_rate as f32;
    if milliseconds < WET_START_MS as f32 {
        0.0
    } else if milliseconds < WET_FULL_MS as f32 {
        smoothstep((milliseconds - WET_START_MS as f32) / (WET_FULL_MS - WET_START_MS) as f32)
    } else if milliseconds < WET_HOLD_END_MS as f32 {
        1.0
    } else if milliseconds < WET_END_MS as f32 {
        1.0 - smoothstep(
            (milliseconds - WET_HOLD_END_MS as f32) / (WET_END_MS - WET_HOLD_END_MS) as f32,
        )
    } else {
        0.0
    }
}

#[inline]
fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn source_before_presentation_is_sample_identical() {
        let sample_rate = 48_000;
        let mut expected = StruckObject::new(StruckTopology::CoupledWire, sample_rate).unwrap();
        let mut voice = ControlledThumpVoice::new(CONFIGS[0], sample_rate).unwrap();
        for _ in 0..voice.duration_frames() {
            let expected = expected.sample();
            let trace = voice.sample_trace();
            assert_eq!(trace.source.left.to_bits(), expected.left.to_bits());
            assert_eq!(trace.source.right.to_bits(), expected.right.to_bits());
        }
    }

    #[test]
    fn sample_path_is_finite_allocation_free_and_returns_to_zero() {
        let mut voice = ControlledThumpVoice::new(CONFIGS[1], 48_000).unwrap();
        assert_no_alloc(|| {
            for _ in 0..voice.duration_frames() {
                let frame = voice.sample();
                assert!(frame.left.is_finite() && frame.right.is_finite());
            }
            assert_eq!(voice.sample(), StereoFrame::default());
        });
    }

    #[test]
    fn nonlinear_residual_is_confined_to_the_declared_window() {
        let sample_rate = 48_000;
        let mut voice = ControlledThumpVoice::new(CONFIGS[2], sample_rate).unwrap();
        for frame in 0..voice.duration_frames() {
            let trace = voice.sample_trace();
            let residual =
                trace.nonlinear_residual.left.abs() + trace.nonlinear_residual.right.abs();
            if frame < WET_START_MS as usize * sample_rate as usize / 1_000
                || frame >= WET_END_MS as usize * sample_rate as usize / 1_000
            {
                assert_eq!(residual.to_bits(), 0.0_f32.to_bits());
            }
        }
    }

    #[test]
    fn least_nonlinear_passing_configuration_meets_every_contract() {
        let selected = select_candidate(48_000).unwrap();
        let evidence = evaluate(&selected);
        assert!(
            evidence.rejection_reasons().is_empty(),
            "{evidence:?}: {:?}",
            evidence.rejection_reasons()
        );
        assert!((-15.5..=-11.5).contains(&evidence.total_rms_dbfs));
        assert!(evidence.sample_peak_dbfs <= -2.0);
        assert!(evidence.true_peak_dbfs <= -1.5);
        assert!((-4.0..=-1.5).contains(&evidence.low_onset_reduction_db));
        assert!(evidence.low_nonlinear_residual_db <= -40.0);
        assert!((-30.0..=-12.0).contains(&evidence.thump_residual_db));
        assert!(evidence.high_rate_residual_db <= -50.0);
    }

    #[test]
    fn deliberately_weak_and_noisy_evidence_is_rejected() {
        let selected = select_candidate(48_000).unwrap();
        let mut evidence = evaluate(&selected);
        evidence.total_rms_dbfs = -30.0;
        evidence.spectral_flatness = 0.9;
        evidence.tonal_pass_fraction = 0.0;
        let reasons = evidence.rejection_reasons();
        assert!(reasons.contains(&ThumpRejection::WeakOrExcessiveRms));
        assert!(reasons.contains(&ThumpRejection::NoiseLike));
    }

    #[test]
    fn selection_is_exactly_deterministic() {
        let first = select_candidate(48_000).unwrap();
        let second = select_candidate(48_000).unwrap();
        assert_eq!(first.config, second.config);
        assert_eq!(first.gain.to_bits(), second.gain.to_bits());
        assert_eq!(
            first
                .samples
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            second
                .samples
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }
}
