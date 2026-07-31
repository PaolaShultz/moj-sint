pub mod algorithm;
pub mod envelope;
pub mod operator;

use thiserror::Error;

use algorithm::{AlgorithmError, CLASSIC_ALGORITHMS, Edge, PreparedAlgorithm};
use operator::SharedSineTable;

pub use envelope::{Envelope, EnvelopeError, EnvelopeSpec};
pub use operator::{
    FrequencyMode, KeyboardScaling, OperatorError, OperatorSpec, PreparedOperator, SineTable,
};

pub const MAX_PM_CYCLES: f32 = 4.0;
pub const MAX_FEEDBACK_CYCLES: f32 = 2.0;
pub const MAX_LFO_RATE_HZ: f32 = 20.0;
pub const MAX_LFO_PITCH_DEPTH_CENTS: f32 = 100.0;
pub const MAX_OUTPUT_GAIN: f32 = 4.0;

const OPERATOR_COUNT: usize = 6;
const STAGE_COUNT: usize = 4;
const OUTPUT_MIN: f32 = -1.0;
const OUTPUT_MAX: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PitchEnvelopeSpec {
    pub seconds: [f32; STAGE_COUNT],
    pub semitones: [f32; STAGE_COUNT],
}

impl PitchEnvelopeSpec {
    pub fn new(
        seconds: [f32; STAGE_COUNT],
        semitones: [f32; STAGE_COUNT],
    ) -> Result<Self, PitchEnvelopeError> {
        let spec = Self { seconds, semitones };
        spec.validate()?;
        Ok(spec)
    }

    fn validate(self) -> Result<(), PitchEnvelopeError> {
        for (stage, seconds) in self.seconds.into_iter().enumerate() {
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err(PitchEnvelopeError::InvalidSeconds { stage });
            }
        }
        for (stage, semitones) in self.semitones.into_iter().enumerate() {
            if !semitones.is_finite() || !(-24.0..=24.0).contains(&semitones) {
                return Err(PitchEnvelopeError::InvalidSemitones { stage });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum PitchEnvelopeError {
    #[error("pitch-envelope stage {stage} has invalid seconds")]
    InvalidSeconds { stage: usize },
    #[error("pitch-envelope stage {stage} has invalid semitones")]
    InvalidSemitones { stage: usize },
    #[error("invalid pitch-envelope sample rate")]
    InvalidSampleRate,
    #[error("pitch-envelope stage {stage} sample count is not representable")]
    SampleCountOutOfRange { stage: usize },
    #[error("pitch-envelope stage {stage} multiplier is not finite and positive")]
    InvalidMultiplier { stage: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PitchStage {
    Idle,
    One,
    Two,
    Three,
    Hold,
    Release,
}

#[derive(Clone, Debug)]
pub struct PitchEnvelope {
    multipliers: [f32; STAGE_COUNT],
    sample_counts: [u32; STAGE_COUNT],
    current: f64,
    target: f64,
    step: f64,
    remaining: u32,
    stage: PitchStage,
}

impl PitchEnvelope {
    pub fn new(spec: PitchEnvelopeSpec, sample_rate: f32) -> Result<Self, PitchEnvelopeError> {
        spec.validate()?;
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(PitchEnvelopeError::InvalidSampleRate);
        }

        let mut sample_counts = [0; STAGE_COUNT];
        for (stage, (&seconds, count)) in spec
            .seconds
            .iter()
            .zip(sample_counts.iter_mut())
            .enumerate()
        {
            let exact = f64::from(seconds) * f64::from(sample_rate);
            if !exact.is_finite() || exact < 1.0 || exact > f64::from(u32::MAX) {
                return Err(PitchEnvelopeError::SampleCountOutOfRange { stage });
            }
            let rounded = exact.round();
            if rounded < 1.0 || rounded > f64::from(u32::MAX) {
                return Err(PitchEnvelopeError::SampleCountOutOfRange { stage });
            }
            *count = rounded as u32;
        }

        let mut multipliers = [0.0; STAGE_COUNT];
        for (stage, (&semitones, multiplier)) in spec
            .semitones
            .iter()
            .zip(multipliers.iter_mut())
            .enumerate()
        {
            *multiplier = 2.0_f32.powf(semitones / 12.0);
            if !multiplier.is_finite() || *multiplier <= 0.0 {
                return Err(PitchEnvelopeError::InvalidMultiplier { stage });
            }
        }

        let initial = f64::from(multipliers[3]);
        Ok(Self {
            multipliers,
            sample_counts,
            current: initial,
            target: initial,
            step: 0.0,
            remaining: 0,
            stage: PitchStage::Idle,
        })
    }

    pub fn note_on(&mut self) {
        self.begin(PitchStage::One, self.multipliers[0], self.sample_counts[0]);
    }

    pub fn note_off(&mut self) {
        if self.stage != PitchStage::Idle {
            self.begin(
                PitchStage::Release,
                self.multipliers[3],
                self.sample_counts[3],
            );
        }
    }

    #[inline]
    pub fn advance(&mut self) -> f32 {
        if self.remaining != 0 {
            self.current += self.step;
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
                self.complete_stage();
            }
        }
        self.current as f32
    }

    pub const fn multiplier(&self) -> f32 {
        self.current as f32
    }

    pub const fn is_idle(&self) -> bool {
        matches!(self.stage, PitchStage::Idle)
    }

    pub fn reset(&mut self) {
        let initial = f64::from(self.multipliers[3]);
        self.current = initial;
        self.target = initial;
        self.step = 0.0;
        self.remaining = 0;
        self.stage = PitchStage::Idle;
    }

    fn begin(&mut self, stage: PitchStage, target: f32, samples: u32) {
        self.stage = stage;
        self.target = f64::from(target);
        self.remaining = samples;
        self.step = (self.target - self.current) / f64::from(samples);
    }

    fn complete_stage(&mut self) {
        match self.stage {
            PitchStage::One => {
                self.begin(PitchStage::Two, self.multipliers[1], self.sample_counts[1])
            }
            PitchStage::Two => self.begin(
                PitchStage::Three,
                self.multipliers[2],
                self.sample_counts[2],
            ),
            PitchStage::Three => {
                self.stage = PitchStage::Hold;
                self.step = 0.0;
            }
            PitchStage::Release => {
                self.stage = PitchStage::Idle;
                self.step = 0.0;
            }
            PitchStage::Idle | PitchStage::Hold => {}
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LfoSpec {
    pub rate_hz: f32,
    pub pitch_depth_cents: f32,
}

impl LfoSpec {
    pub fn new(rate_hz: f32, pitch_depth_cents: f32) -> Result<Self, LfoError> {
        let spec = Self {
            rate_hz,
            pitch_depth_cents,
        };
        spec.validate()?;
        Ok(spec)
    }

    fn validate(self) -> Result<(), LfoError> {
        if !self.rate_hz.is_finite() || !(0.0..=MAX_LFO_RATE_HZ).contains(&self.rate_hz) {
            return Err(LfoError::InvalidRate);
        }
        if !self.pitch_depth_cents.is_finite()
            || !(0.0..=MAX_LFO_PITCH_DEPTH_CENTS).contains(&self.pitch_depth_cents)
        {
            return Err(LfoError::InvalidPitchDepth);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum LfoError {
    #[error("invalid LFO rate")]
    InvalidRate,
    #[error("invalid LFO pitch depth")]
    InvalidPitchDepth,
    #[error("prepared LFO rotation is not finite")]
    InvalidRotation,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SixOpPatch {
    pub algorithm: usize,
    pub operators: [OperatorSpec; OPERATOR_COUNT],
    pub pitch_envelope: PitchEnvelopeSpec,
    pub lfo: LfoSpec,
    pub feedback: f32,
    pub output_gain: f32,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum VoiceError {
    #[error("invalid classic algorithm index")]
    InvalidAlgorithm,
    #[error("invalid sample rate")]
    InvalidSampleRate,
    #[error("invalid MIDI note")]
    InvalidNote,
    #[error("invalid velocity")]
    InvalidVelocity,
    #[error("feedback must be finite and normalized")]
    InvalidFeedback,
    #[error("output gain must be finite and within its architectural maximum")]
    InvalidOutputGain,
    #[error(transparent)]
    PitchEnvelope(#[from] PitchEnvelopeError),
    #[error(transparent)]
    Lfo(#[from] LfoError),
    #[error(transparent)]
    Algorithm(#[from] AlgorithmError),
    #[error("operator {operator} could not be prepared: {source}")]
    Operator {
        operator: u8,
        #[source]
        source: OperatorError,
    },
}

#[derive(Clone, Debug)]
pub struct SixOpVoice {
    algorithm: PreparedAlgorithm,
    operators: [PreparedOperator; OPERATOR_COUNT],
    outputs: [f32; OPERATOR_COUNT],
    feedback_sample: f32,
    feedback_amount: f32,
    pitch_envelope: PitchEnvelope,
    lfo_sine: f32,
    lfo_cosine: f32,
    lfo_rotation_sine: f32,
    lfo_rotation_cosine: f32,
    lfo_pitch_depth_ratio: f32,
    output_gain: f32,
    non_finite_seen: bool,
    clamp_contacts: u64,
}

impl SixOpVoice {
    pub fn new(
        patch: SixOpPatch,
        sample_rate: f32,
        note: u8,
        velocity: f32,
    ) -> Result<Self, VoiceError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(VoiceError::InvalidSampleRate);
        }
        if note > 127 {
            return Err(VoiceError::InvalidNote);
        }
        if !velocity.is_finite() || !(0.0..=1.0).contains(&velocity) {
            return Err(VoiceError::InvalidVelocity);
        }
        let algorithm_spec = CLASSIC_ALGORITHMS
            .get(patch.algorithm)
            .copied()
            .ok_or(VoiceError::InvalidAlgorithm)?;
        if !patch.feedback.is_finite() || !(0.0..=1.0).contains(&patch.feedback) {
            return Err(VoiceError::InvalidFeedback);
        }
        if !patch.output_gain.is_finite() || !(0.0..=MAX_OUTPUT_GAIN).contains(&patch.output_gain) {
            return Err(VoiceError::InvalidOutputGain);
        }
        patch.lfo.validate()?;

        let pitch_envelope = PitchEnvelope::new(patch.pitch_envelope, sample_rate)?;
        let lfo_angle = std::f32::consts::TAU * patch.lfo.rate_hz / sample_rate;
        if !lfo_angle.is_finite() {
            return Err(LfoError::InvalidRotation.into());
        }
        let (lfo_rotation_sine, lfo_rotation_cosine) = lfo_angle.sin_cos();
        let lfo_pitch_depth_ratio = 2.0_f32.powf(patch.lfo.pitch_depth_cents / 1_200.0) - 1.0;
        if !lfo_rotation_sine.is_finite()
            || !lfo_rotation_cosine.is_finite()
            || !lfo_pitch_depth_ratio.is_finite()
        {
            return Err(LfoError::InvalidRotation.into());
        }

        let algorithm = PreparedAlgorithm::new(algorithm_spec)?;
        let table: SharedSineTable = SineTable::new().into();
        let [
            operator_1,
            operator_2,
            operator_3,
            operator_4,
            operator_5,
            operator_6,
        ] = patch.operators;
        let operators = [
            prepare_operator(operator_1, 1, sample_rate, note, velocity, table.clone())?,
            prepare_operator(operator_2, 2, sample_rate, note, velocity, table.clone())?,
            prepare_operator(operator_3, 3, sample_rate, note, velocity, table.clone())?,
            prepare_operator(operator_4, 4, sample_rate, note, velocity, table.clone())?,
            prepare_operator(operator_5, 5, sample_rate, note, velocity, table.clone())?,
            prepare_operator(operator_6, 6, sample_rate, note, velocity, table)?,
        ];

        Ok(Self {
            algorithm,
            operators,
            outputs: [0.0; OPERATOR_COUNT],
            feedback_sample: 0.0,
            feedback_amount: patch.feedback,
            pitch_envelope,
            lfo_sine: 0.0,
            lfo_cosine: 1.0,
            lfo_rotation_sine,
            lfo_rotation_cosine,
            lfo_pitch_depth_ratio,
            output_gain: patch.output_gain,
            non_finite_seen: false,
            clamp_contacts: 0,
        })
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        let pitch_envelope = self.pitch_envelope.advance();
        let lfo_finite = self.advance_lfo();
        let lfo_multiplier = 1.0 + self.lfo_sine * self.lfo_pitch_depth_ratio;
        let mut pitch_multiplier = pitch_envelope * lfo_multiplier;
        let mut non_finite =
            !lfo_finite || !pitch_multiplier.is_finite() || pitch_multiplier <= 0.0;
        if non_finite {
            self.pitch_envelope.reset();
            pitch_multiplier = 1.0;
        }

        let order = *self.algorithm.evaluation_order();
        let feedback = self.algorithm.feedback();
        for operator in order {
            let operator_index = usize::from(operator);
            let mut phase_modulation = 0.0;
            if let Some(incoming) = self.algorithm.incoming(operator + 1) {
                for &source in incoming {
                    phase_modulation += self.outputs[usize::from(source)] * MAX_PM_CYCLES;
                }
            }
            if operator == feedback.target {
                phase_modulation +=
                    self.feedback_sample * self.feedback_amount * MAX_FEEDBACK_CYCLES;
            }
            if !phase_modulation.is_finite() {
                phase_modulation = 0.0;
                non_finite = true;
            }

            let phase_was_finite = self.operators[operator_index].recover_phase_if_non_finite();
            let output = self.operators[operator_index].sample(phase_modulation, pitch_multiplier);
            let phase_is_finite = self.operators[operator_index].recover_phase_if_non_finite();
            if !phase_was_finite || !phase_is_finite || !output.is_finite() {
                self.outputs[operator_index] = 0.0;
                non_finite = true;
            } else {
                self.outputs[operator_index] = output;
            }
        }

        let next_feedback = self.outputs[usize::from(feedback.source)];
        if next_feedback.is_finite() {
            self.feedback_sample = next_feedback;
        } else {
            self.feedback_sample = 0.0;
            non_finite = true;
        }

        let mut carrier_sum = 0.0;
        for (operator, &output) in self.outputs.iter().enumerate() {
            if self.algorithm.is_carrier(operator as u8 + 1) == Some(true) {
                carrier_sum += output;
            }
        }
        let output = carrier_sum * self.algorithm.output_gain() * self.output_gain;
        if non_finite || !output.is_finite() {
            self.non_finite_seen = true;
            return 0.0;
        }
        if !(OUTPUT_MIN..=OUTPUT_MAX).contains(&output) {
            self.clamp_contacts = self.clamp_contacts.saturating_add(1);
            return output.clamp(OUTPUT_MIN, OUTPUT_MAX);
        }
        output
    }

    pub fn note_on(&mut self) {
        for operator in &mut self.operators {
            operator.note_on();
        }
        self.pitch_envelope.note_on();
        self.outputs = [0.0; OPERATOR_COUNT];
        self.feedback_sample = 0.0;
        self.lfo_sine = 0.0;
        self.lfo_cosine = 1.0;
    }

    pub fn note_off(&mut self) {
        for operator in &mut self.operators {
            operator.note_off();
        }
        self.pitch_envelope.note_off();
    }

    pub fn reset(&mut self) {
        for operator in &mut self.operators {
            operator.reset();
        }
        self.outputs = [0.0; OPERATOR_COUNT];
        self.feedback_sample = 0.0;
        self.pitch_envelope.reset();
        self.lfo_sine = 0.0;
        self.lfo_cosine = 1.0;
        self.non_finite_seen = false;
        self.clamp_contacts = 0;
    }

    pub fn is_idle(&self) -> bool {
        self.pitch_envelope.is_idle() && self.operators.iter().all(PreparedOperator::is_idle)
    }

    pub const fn non_finite_seen(&self) -> bool {
        self.non_finite_seen
    }

    pub const fn clamp_contacts(&self) -> u64 {
        self.clamp_contacts
    }

    pub const fn feedback_sample(&self) -> f32 {
        self.feedback_sample
    }

    pub const fn feedback_edge(&self) -> Edge {
        self.algorithm.feedback()
    }

    pub fn evaluation_position(&self, operator: u8) -> Option<usize> {
        self.algorithm.position(operator)
    }

    pub fn operator_output(&self, operator: u8) -> Option<f32> {
        let index = one_based_operator_index(operator)?;
        Some(self.outputs[index])
    }

    pub fn operator_phase(&self, operator: u8) -> Option<f32> {
        let index = one_based_operator_index(operator)?;
        Some(self.operators[index].phase())
    }

    pub const fn pitch_envelope_multiplier(&self) -> f32 {
        self.pitch_envelope.multiplier()
    }

    pub const fn lfo_sine(&self) -> f32 {
        self.lfo_sine
    }

    #[inline]
    fn advance_lfo(&mut self) -> bool {
        let sine =
            self.lfo_sine * self.lfo_rotation_cosine + self.lfo_cosine * self.lfo_rotation_sine;
        let cosine =
            self.lfo_cosine * self.lfo_rotation_cosine - self.lfo_sine * self.lfo_rotation_sine;
        let norm_squared = sine * sine + cosine * cosine;
        let correction = 1.5 - 0.5 * norm_squared;
        let corrected_sine = sine * correction;
        let corrected_cosine = cosine * correction;
        if corrected_sine.is_finite() && corrected_cosine.is_finite() {
            self.lfo_sine = corrected_sine;
            self.lfo_cosine = corrected_cosine;
            true
        } else {
            self.lfo_sine = 0.0;
            self.lfo_cosine = 1.0;
            self.non_finite_seen = true;
            false
        }
    }
}

fn prepare_operator(
    spec: OperatorSpec,
    operator: u8,
    sample_rate: f32,
    note: u8,
    velocity: f32,
    table: SharedSineTable,
) -> Result<PreparedOperator, VoiceError> {
    PreparedOperator::new(spec, sample_rate, note, velocity, table)
        .map_err(|source| VoiceError::Operator { operator, source })
}

fn one_based_operator_index(operator: u8) -> Option<usize> {
    let index = usize::from(operator.checked_sub(1)?);
    (index < OPERATOR_COUNT).then_some(index)
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;

    use assert_no_alloc::assert_no_alloc;

    use super::*;
    use crate::dsp::six_op_pm::algorithm::{CLASSIC_ALGORITHMS, Edge};

    const SAMPLE_RATE: f32 = 48_000.0;

    fn amplitude_at(samples: &[f32], frequency_hz: f32, sample_rate: f32) -> f32 {
        let mut real = 0.0_f64;
        let mut imaginary = 0.0_f64;
        for (index, &sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * f64::from(frequency_hz) * index as f64
                / f64::from(sample_rate);
            real += f64::from(sample) * phase.cos();
            imaginary -= f64::from(sample) * phase.sin();
        }
        (2.0 * real.hypot(imaginary) / samples.len() as f64) as f32
    }

    fn amplitude_envelope() -> EnvelopeSpec {
        EnvelopeSpec::new([0.000_1; 4], [1.0, 1.0, 1.0, 0.0]).unwrap()
    }

    fn operator(frequency_hz: f32, level: f32) -> OperatorSpec {
        OperatorSpec::new(
            FrequencyMode::fixed(frequency_hz).unwrap(),
            level,
            amplitude_envelope(),
            0.0,
            KeyboardScaling::new(69, 0.0, 0.0).unwrap(),
            0.0,
        )
        .unwrap()
    }

    fn patch(algorithm: usize) -> SixOpPatch {
        SixOpPatch {
            algorithm,
            operators: [
                operator(1_000.0, 0.4),
                operator(250.0, 0.0),
                operator(300.0, 0.0),
                operator(400.0, 0.0),
                operator(500.0, 0.0),
                operator(600.0, 0.0),
            ],
            pitch_envelope: PitchEnvelopeSpec::new([0.001; 4], [0.0; 4]).unwrap(),
            lfo: LfoSpec::new(0.0, 0.0).unwrap(),
            feedback: 0.0,
            output_gain: 0.5,
        }
    }

    fn render(voice: &mut SixOpVoice, count: usize) -> Vec<f32> {
        (0..count).map(|_| voice.sample()).collect()
    }

    #[test]
    fn one_carrier_retains_expected_frequency_and_amplitude() {
        let mut patch = patch(9);
        patch.operators[0] = operator(1_000.0, 0.8);
        let mut voice = SixOpVoice::new(patch, SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        render(&mut voice, 64);
        let samples = render(&mut voice, SAMPLE_RATE as usize);

        let carrier = amplitude_at(&samples, 1_000.0, SAMPLE_RATE);
        let below = amplitude_at(&samples, 900.0, SAMPLE_RATE);
        let above = amplitude_at(&samples, 1_100.0, SAMPLE_RATE);
        assert!((carrier - 0.3).abs() < 2.0e-3, "carrier={carrier}");
        assert!(below < 1.0e-4, "below={below}");
        assert!(above < 1.0e-4, "above={above}");
        assert!(!voice.non_finite_seen());
        assert_eq!(voice.clamp_contacts(), 0);
    }

    #[test]
    fn two_operator_pm_has_carrier_and_symmetric_sidebands() {
        let mut patch = patch(11);
        patch.operators[0] = operator(1_000.0, 0.65);
        patch.operators[1] = operator(250.0, 0.05);
        patch.output_gain = 0.4;
        let mut voice = SixOpVoice::new(patch, SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        render(&mut voice, 64);
        let samples = render(&mut voice, SAMPLE_RATE as usize);

        let carrier = amplitude_at(&samples, 1_000.0, SAMPLE_RATE);
        let lower = amplitude_at(&samples, 750.0, SAMPLE_RATE);
        let upper = amplitude_at(&samples, 1_250.0, SAMPLE_RATE);
        assert!(carrier > 0.08, "carrier={carrier}");
        assert!(lower > 0.03, "lower={lower}");
        assert!(upper > 0.03, "upper={upper}");
        assert!((lower - upper).abs() / lower.max(upper) < 0.08);
        assert!(!voice.non_finite_seen());
        assert_eq!(voice.clamp_contacts(), 0);
    }

    #[test]
    fn graph_four_feedback_uses_previous_operator_four_output_at_operator_six() {
        let mut patch = patch(3);
        patch.operators = [
            operator(100.0, 0.0),
            operator(200.0, 0.0),
            operator(300.0, 0.0),
            operator(400.0, 0.25),
            operator(500.0, 0.25),
            operator(600.0, 0.25),
        ];
        patch.feedback = 1.0;
        patch.output_gain = 0.25;
        let mut voice = SixOpVoice::new(patch, SAMPLE_RATE, 69, 1.0).unwrap();
        assert_eq!(voice.feedback_edge(), Edge::one_based(4, 6));
        assert!(voice.evaluation_position(6).unwrap() < voice.evaluation_position(5).unwrap());
        assert!(voice.evaluation_position(5).unwrap() < voice.evaluation_position(4).unwrap());

        voice.note_on();
        voice.sample();
        voice.sample();
        let delayed = voice.feedback_sample();
        assert_eq!(delayed, voice.operator_output(4).unwrap());
        assert_ne!(delayed, 0.0);

        let expected = SineTable::lookup(
            &SineTable::new(),
            2.0 * 600.0 / SAMPLE_RATE + delayed * MAX_FEEDBACK_CYCLES,
        ) * 0.25
            * 0.6_f32.powi(2);
        let unmodulated = SineTable::lookup(&SineTable::new(), 2.0 * 600.0 / SAMPLE_RATE)
            * 0.25
            * 0.6_f32.powi(2);
        voice.sample();
        let actual = voice.operator_output(6).unwrap();
        assert!((actual - expected).abs() < 1.0e-6, "{actual} != {expected}");
        assert!((actual - unmodulated).abs() > 1.0e-3);
    }

    #[test]
    fn maximum_self_and_group_feedback_are_finite_bounded_and_deterministic() {
        for algorithm in [0, 3] {
            let mut first_patch = patch(algorithm);
            first_patch.operators = [
                operator(110.0, 0.2),
                operator(170.0, 0.2),
                operator(230.0, 0.2),
                operator(290.0, 0.2),
                operator(350.0, 0.2),
                operator(410.0, 0.2),
            ];
            first_patch.feedback = 1.0;
            first_patch.output_gain = 0.2;
            let second_patch = first_patch;
            let mut first = SixOpVoice::new(first_patch, SAMPLE_RATE, 48, 0.9).unwrap();
            let mut second = SixOpVoice::new(second_patch, SAMPLE_RATE, 48, 0.9).unwrap();
            first.note_on();
            second.note_on();
            for _ in 0..24_000 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(a.is_finite() && a.abs() <= 1.0);
            }
            assert!(!first.non_finite_seen());
            assert_eq!(first.clamp_contacts(), 0);
        }
    }

    #[test]
    fn pitch_envelope_and_lfo_are_bounded_resettable_and_common_to_all_operators() {
        let mut moving_patch = patch(31);
        moving_patch.operators = [operator(100.0, 0.1); 6];
        moving_patch.pitch_envelope =
            PitchEnvelopeSpec::new([0.01; 4], [12.0, -12.0, 6.0, 0.0]).unwrap();
        moving_patch.lfo = LfoSpec::new(6.7, 5.0).unwrap();
        moving_patch.output_gain = 0.1;
        let mut static_patch = moving_patch;
        static_patch.pitch_envelope = PitchEnvelopeSpec::new([0.01; 4], [0.0; 4]).unwrap();
        static_patch.lfo = LfoSpec::new(0.0, 0.0).unwrap();

        let mut moving = SixOpVoice::new(moving_patch, SAMPLE_RATE, 60, 1.0).unwrap();
        let mut stationary = SixOpVoice::new(static_patch, SAMPLE_RATE, 60, 1.0).unwrap();
        moving.note_on();
        stationary.note_on();
        let first = render(&mut moving, 1_024);
        render(&mut stationary, 1_024);

        assert_ne!(moving.operator_phase(1), stationary.operator_phase(1));
        for operator in 2..=6 {
            assert_eq!(moving.operator_phase(operator), moving.operator_phase(1));
        }
        assert!((0.5..=2.0).contains(&moving.pitch_envelope_multiplier()));
        assert!(moving.lfo_sine().abs() <= 1.001);
        assert!(!moving.non_finite_seen());

        moving.reset();
        moving.note_on();
        assert_eq!(render(&mut moving, 1_024), first);
    }

    #[test]
    fn note_lifecycle_and_rapid_events_remain_finite_and_return_to_idle() {
        let mut voice = SixOpVoice::new(patch(3), SAMPLE_RATE, 69, 1.0).unwrap();
        assert!(voice.is_idle());
        for index in 0..1_000 {
            if index % 7 == 0 {
                voice.note_on();
            }
            if index % 11 == 0 {
                voice.note_off();
            }
            let sample = voice.sample();
            assert!(sample.is_finite() && sample.abs() <= 1.0);
        }
        voice.note_on();
        render(&mut voice, 32);
        voice.note_off();
        render(&mut voice, 64);
        assert!(voice.is_idle());
        assert!(!voice.non_finite_seen());
    }

    #[test]
    fn reset_replays_bit_identical_samples_and_diagnostics() {
        let mut patch = patch(3);
        patch.feedback = 0.8;
        patch.lfo = LfoSpec::new(5.0, 4.0).unwrap();
        let mut voice = SixOpVoice::new(patch, SAMPLE_RATE, 52, 0.75).unwrap();
        voice.note_on();
        let first = render(&mut voice, 4_096);
        voice.note_off();
        render(&mut voice, 32);

        voice.reset();
        assert_eq!(voice.feedback_sample(), 0.0);
        assert_eq!(voice.clamp_contacts(), 0);
        assert!(!voice.non_finite_seen());
        voice.note_on();
        assert_eq!(render(&mut voice, 4_096), first);
    }

    #[test]
    fn invalid_voice_patch_pitch_lfo_and_note_inputs_are_rejected_explicitly() {
        assert!(matches!(
            SixOpVoice::new(patch(CLASSIC_ALGORITHMS.len()), SAMPLE_RATE, 69, 1.0),
            Err(VoiceError::InvalidAlgorithm)
        ));
        for sample_rate in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                SixOpVoice::new(patch(0), sample_rate, 69, 1.0),
                Err(VoiceError::InvalidSampleRate)
            ));
        }
        assert!(matches!(
            SixOpVoice::new(patch(0), SAMPLE_RATE, 128, 1.0),
            Err(VoiceError::InvalidNote)
        ));
        for velocity in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                SixOpVoice::new(patch(0), SAMPLE_RATE, 69, velocity),
                Err(VoiceError::InvalidVelocity)
            ));
        }

        for seconds in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut invalid = patch(0);
            invalid.pitch_envelope.seconds[2] = seconds;
            assert!(matches!(
                SixOpVoice::new(invalid, SAMPLE_RATE, 69, 1.0),
                Err(VoiceError::PitchEnvelope(_))
            ));
        }
        for semitones in [-24.01, 24.01, f32::NAN, f32::INFINITY] {
            let mut invalid = patch(0);
            invalid.pitch_envelope.semitones[1] = semitones;
            assert!(matches!(
                SixOpVoice::new(invalid, SAMPLE_RATE, 69, 1.0),
                Err(VoiceError::PitchEnvelope(_))
            ));
        }
        let mut unrepresentable = patch(0);
        unrepresentable.pitch_envelope.seconds = [f32::MAX; 4];
        assert!(matches!(
            SixOpVoice::new(unrepresentable, SAMPLE_RATE, 69, 1.0),
            Err(VoiceError::PitchEnvelope(_))
        ));

        for (rate, depth) in [
            (-0.01, 0.0),
            (MAX_LFO_RATE_HZ + 0.01, 0.0),
            (f32::NAN, 0.0),
            (0.0, -0.01),
            (0.0, MAX_LFO_PITCH_DEPTH_CENTS + 0.01),
            (0.0, f32::INFINITY),
        ] {
            let mut invalid = patch(0);
            invalid.lfo = LfoSpec {
                rate_hz: rate,
                pitch_depth_cents: depth,
            };
            assert!(matches!(
                SixOpVoice::new(invalid, SAMPLE_RATE, 69, 1.0),
                Err(VoiceError::Lfo(_))
            ));
        }

        for feedback in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
            let mut invalid = patch(0);
            invalid.feedback = feedback;
            assert!(matches!(
                SixOpVoice::new(invalid, SAMPLE_RATE, 69, 1.0),
                Err(VoiceError::InvalidFeedback)
            ));
        }
        for gain in [-0.01, MAX_OUTPUT_GAIN + 0.01, f32::NAN, f32::INFINITY] {
            let mut invalid = patch(0);
            invalid.output_gain = gain;
            assert!(matches!(
                SixOpVoice::new(invalid, SAMPLE_RATE, 69, 1.0),
                Err(VoiceError::InvalidOutputGain)
            ));
        }
        let mut invalid_operator = patch(0);
        invalid_operator.operators[2].output_level = f32::NAN;
        assert!(matches!(
            SixOpVoice::new(invalid_operator, SAMPLE_RATE, 69, 1.0),
            Err(VoiceError::Operator { operator: 3, .. })
        ));
    }

    #[test]
    fn emergency_output_guard_counts_contacts_deterministically() {
        let mut loud_patch = patch(31);
        loud_patch.operators = [operator(1_000.0, 1.0); 6];
        loud_patch.output_gain = MAX_OUTPUT_GAIN;
        let second_patch = loud_patch;
        let mut first = SixOpVoice::new(loud_patch, SAMPLE_RATE, 69, 1.0).unwrap();
        let mut second = SixOpVoice::new(second_patch, SAMPLE_RATE, 69, 1.0).unwrap();
        first.note_on();
        second.note_on();
        for _ in 0..2_000 {
            let a = first.sample();
            assert_eq!(a, second.sample());
            assert!(a.is_finite() && a.abs() <= 1.0);
        }
        assert!(first.clamp_contacts() > 0);
        assert_eq!(first.clamp_contacts(), second.clamp_contacts());
        assert!(!first.non_finite_seen());
    }

    #[test]
    fn unexpected_internal_non_finite_is_silenced_latched_and_does_not_poison_state() {
        let mut voice = SixOpVoice::new(patch(9), SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        render(&mut voice, 32);
        voice.lfo_sine = f32::NAN;

        assert_eq!(voice.sample(), 0.0);
        assert!(voice.non_finite_seen());
        let recovered = voice.sample();
        assert!(recovered.is_finite() && recovered.abs() <= 1.0);
    }

    #[test]
    fn poisoned_pitch_envelope_is_silenced_latched_and_recovers_next_sample() {
        let mut voice = SixOpVoice::new(patch(9), SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        render(&mut voice, 32);
        voice.pitch_envelope.current = f64::NAN;

        assert_eq!(voice.sample(), 0.0);
        assert!(voice.non_finite_seen());
        let recovered = voice.sample();
        assert!(recovered.is_finite());
        assert_ne!(recovered, 0.0);
    }

    #[test]
    fn poisoned_operator_phase_is_silenced_latched_and_recovers_next_sample() {
        let mut voice = SixOpVoice::new(patch(9), SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        render(&mut voice, 32);
        voice.operators[0].poison_phase_for_test();

        assert_eq!(voice.sample(), 0.0);
        assert!(voice.non_finite_seen());
        let recovered = voice.sample();
        assert!(recovered.is_finite());
        assert_ne!(recovered, 0.0);
    }

    #[test]
    fn sample_path_is_allocation_free_and_contains_no_transcendental_setup() {
        let mut voice = SixOpVoice::new(patch(3), SAMPLE_RATE, 69, 1.0).unwrap();
        voice.note_on();
        assert_no_alloc(|| {
            for _ in 0..48_000 {
                black_box(voice.sample());
            }
        });

        let source = include_str!("mod.rs");
        let sample_path = source
            .split("pub fn sample(&mut self) -> f32")
            .nth(1)
            .unwrap()
            .split("pub fn note_on")
            .next()
            .unwrap();
        for forbidden in [".sin(", ".cos(", "sin_cos(", ".powf(", ".sqrt("] {
            assert!(
                !sample_path.contains(forbidden),
                "sample path contains {forbidden}"
            );
        }
    }
}
