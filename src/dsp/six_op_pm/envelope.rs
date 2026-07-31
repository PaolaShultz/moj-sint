use thiserror::Error;

const STAGE_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeSpec {
    pub seconds: [f32; STAGE_COUNT],
    pub levels: [f32; STAGE_COUNT],
}

impl EnvelopeSpec {
    pub fn new(
        seconds: [f32; STAGE_COUNT],
        levels: [f32; STAGE_COUNT],
    ) -> Result<Self, EnvelopeError> {
        let spec = Self { seconds, levels };
        spec.validate()?;
        Ok(spec)
    }

    pub(crate) fn validate(self) -> Result<(), EnvelopeError> {
        for (stage, seconds) in self.seconds.into_iter().enumerate() {
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err(EnvelopeError::InvalidSeconds { stage });
            }
        }
        for (stage, level) in self.levels.into_iter().enumerate() {
            if !level.is_finite() || !(0.0..=1.0).contains(&level) {
                return Err(EnvelopeError::InvalidLevel { stage });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum EnvelopeError {
    #[error("stage {stage} has invalid seconds")]
    InvalidSeconds { stage: usize },
    #[error("stage {stage} has invalid level")]
    InvalidLevel { stage: usize },
    #[error("invalid sample rate")]
    InvalidSampleRate,
    #[error("stage {stage} sample count is not representable")]
    SampleCountOutOfRange { stage: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Idle,
    One,
    Two,
    Three,
    Hold,
    Release,
}

#[derive(Clone, Debug)]
pub struct Envelope {
    spec: EnvelopeSpec,
    sample_counts: [u32; STAGE_COUNT],
    current: f64,
    target: f64,
    step: f64,
    remaining: u32,
    stage: Stage,
}

impl Envelope {
    pub fn new(spec: EnvelopeSpec, sample_rate: f32) -> Result<Self, EnvelopeError> {
        spec.validate()?;
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EnvelopeError::InvalidSampleRate);
        }

        let mut sample_counts = [0; STAGE_COUNT];
        for (stage, (&seconds, sample_count)) in spec
            .seconds
            .iter()
            .zip(sample_counts.iter_mut())
            .enumerate()
        {
            let exact = f64::from(seconds) * f64::from(sample_rate);
            if !exact.is_finite() || exact < 1.0 || exact > f64::from(u32::MAX) {
                return Err(EnvelopeError::SampleCountOutOfRange { stage });
            }
            let rounded = exact.round();
            if rounded < 1.0 || rounded > f64::from(u32::MAX) {
                return Err(EnvelopeError::SampleCountOutOfRange { stage });
            }
            *sample_count = rounded as u32;
        }

        let initial = f64::from(spec.levels[3]);
        Ok(Self {
            spec,
            sample_counts,
            current: initial,
            target: initial,
            step: 0.0,
            remaining: 0,
            stage: Stage::Idle,
        })
    }

    pub fn note_on(&mut self) {
        self.begin(Stage::One, self.spec.levels[0], self.sample_counts[0]);
    }

    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle {
            self.begin(Stage::Release, self.spec.levels[3], self.sample_counts[3]);
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
        (self.current * self.current) as f32
    }

    pub const fn level(&self) -> f32 {
        self.current as f32
    }

    pub const fn is_idle(&self) -> bool {
        matches!(self.stage, Stage::Idle)
    }

    pub fn reset(&mut self) {
        let initial = f64::from(self.spec.levels[3]);
        self.current = initial;
        self.target = initial;
        self.step = 0.0;
        self.remaining = 0;
        self.stage = Stage::Idle;
    }

    pub(super) fn recover_finite_state(&mut self) -> bool {
        if self.current.is_finite() {
            return true;
        }

        let maximum_remaining = match self.stage {
            Stage::Idle | Stage::Hold => 0,
            Stage::One => self.sample_counts[0],
            Stage::Two => self.sample_counts[1],
            Stage::Three => self.sample_counts[2],
            Stage::Release => self.sample_counts[3],
        };
        let reconstructed = self.target - self.step * f64::from(self.remaining);
        if self.target.is_finite()
            && (0.0..=1.0).contains(&self.target)
            && self.step.is_finite()
            && self.remaining <= maximum_remaining
            && reconstructed.is_finite()
            && (0.0..=1.0).contains(&reconstructed)
        {
            self.current = reconstructed;
        } else {
            self.reset();
        }
        false
    }

    #[cfg(test)]
    pub(super) fn poison_current_for_test(&mut self) {
        self.current = f64::NAN;
    }

    fn begin(&mut self, stage: Stage, target: f32, samples: u32) {
        self.stage = stage;
        self.target = f64::from(target);
        self.remaining = samples;
        self.step = (self.target - self.current) / f64::from(samples);
    }

    fn complete_stage(&mut self) {
        match self.stage {
            Stage::One => self.begin(Stage::Two, self.spec.levels[1], self.sample_counts[1]),
            Stage::Two => self.begin(Stage::Three, self.spec.levels[2], self.sample_counts[2]),
            Stage::Three => {
                self.stage = Stage::Hold;
                self.step = 0.0;
            }
            Stage::Release => {
                self.stage = Stage::Idle;
                self.step = 0.0;
            }
            Stage::Idle | Stage::Hold => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 1.0e-6,
            "{actual} != {expected}"
        );
    }

    fn spec() -> EnvelopeSpec {
        EnvelopeSpec::new([0.2; 4], [1.0, 0.8, 0.5, 0.1]).unwrap()
    }

    #[test]
    fn four_stages_complete_on_exact_sample_counts_and_release_to_idle() {
        let mut envelope = Envelope::new(spec(), 10.0).unwrap();
        assert_close(envelope.level(), 0.1);
        assert!(envelope.is_idle());

        envelope.note_on();
        assert_close(envelope.advance(), 0.55_f32.powi(2));
        assert_close(envelope.advance(), 1.0);
        assert_close(envelope.level(), 1.0);

        assert_close(envelope.advance(), 0.9_f32.powi(2));
        assert_close(envelope.advance(), 0.8_f32.powi(2));
        assert_close(envelope.level(), 0.8);

        assert_close(envelope.advance(), 0.65_f32.powi(2));
        assert_close(envelope.advance(), 0.5_f32.powi(2));
        assert_close(envelope.level(), 0.5);
        assert_close(envelope.advance(), 0.5_f32.powi(2));
        assert!(!envelope.is_idle());

        envelope.note_off();
        assert_close(envelope.advance(), 0.3_f32.powi(2));
        assert_close(envelope.advance(), 0.1_f32.powi(2));
        assert_close(envelope.level(), 0.1);
        assert!(envelope.is_idle());
    }

    #[test]
    fn reset_restores_exact_initial_state_and_replay() {
        let mut envelope = Envelope::new(spec(), 10.0).unwrap();
        envelope.note_on();
        let first = envelope.advance();
        for _ in 0..5 {
            envelope.advance();
        }
        envelope.note_off();
        envelope.advance();

        envelope.reset();
        assert_close(envelope.level(), 0.1);
        assert!(envelope.is_idle());
        envelope.note_on();
        assert_eq!(envelope.advance(), first);
    }

    #[test]
    fn invalid_specs_and_sample_rates_are_rejected_before_sampling() {
        for seconds in [
            [0.0, 1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0, 1.0],
            [f32::NAN, 1.0, 1.0, 1.0],
            [f32::INFINITY, 1.0, 1.0, 1.0],
        ] {
            assert!(EnvelopeSpec::new(seconds, [1.0; 4]).is_err());
        }

        for levels in [
            [-0.01, 0.0, 0.0, 0.0],
            [1.01, 0.0, 0.0, 0.0],
            [f32::NAN, 0.0, 0.0, 0.0],
            [f32::INFINITY, 0.0, 0.0, 0.0],
        ] {
            assert!(EnvelopeSpec::new([1.0; 4], levels).is_err());
        }

        for sample_rate in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(Envelope::new(spec(), sample_rate).is_err());
        }

        let unrepresentable = EnvelopeSpec::new([f32::MAX; 4], [1.0; 4]).unwrap();
        assert!(Envelope::new(unrepresentable, f32::MAX).is_err());
        let zero_sample_count = EnvelopeSpec::new([f32::MIN_POSITIVE; 4], [1.0; 4]).unwrap();
        assert!(Envelope::new(zero_sample_count, 48_000.0).is_err());
    }

    #[test]
    fn maximum_representable_sample_count_is_accepted() {
        let spec = EnvelopeSpec::new([65_535.0; 4], [1.0, 0.8, 0.5, 0.1]).unwrap();
        let envelope = Envelope::new(spec, 65_537.0).unwrap();

        assert_eq!(envelope.level(), 0.1);
        assert!(envelope.is_idle());
    }

    #[test]
    fn long_stage_retains_measurable_linear_progress() {
        let spec = EnvelopeSpec::new([0.001, 1_000.0, 0.001, 0.001], [1.0, 0.8, 0.5, 0.0]).unwrap();
        let mut envelope = Envelope::new(spec, 48_000.0).unwrap();
        envelope.note_on();
        for _ in 0..48 {
            envelope.advance();
        }
        assert_eq!(envelope.level(), 1.0);

        for _ in 0..1_000 {
            envelope.advance();
        }
        let expected = 1.0 + (0.8 - 1.0) * 1_000.0 / 48_000_000.0;
        assert!(envelope.level() < 1.0);
        assert_close(envelope.level(), expected);
    }
}
