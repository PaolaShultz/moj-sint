use super::ModelDError;

const MIN_DRIVE: f32 = 1.0;
const MAX_DRIVE: f32 = 4.0;
const CUBIC_LINEAR_GAIN: f32 = 1.5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MixerMode {
    Linear,
    Nonlinear,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MixerConfig {
    pub source_levels: [f32; 3],
    pub feedback_level: f32,
    /// Prepared nonlinear input gain. Valid values are in the inclusive 1 to 4 range.
    pub drive: f32,
    pub mode: MixerMode,
}

/// Prepared scalar mixer for the three oscillator and feedback paths.
#[derive(Clone, Copy, Debug)]
pub struct ModelDMixer {
    source_levels: [f32; 3],
    feedback_level: f32,
    drive: f32,
    mode: MixerMode,
}

impl ModelDMixer {
    pub fn new(config: MixerConfig) -> Result<Self, ModelDError> {
        if config
            .source_levels
            .iter()
            .any(|level| !level.is_finite() || !(0.0..=1.0).contains(level))
            || !config.feedback_level.is_finite()
            || !(0.0..=1.0).contains(&config.feedback_level)
            || !config.drive.is_finite()
            || !(MIN_DRIVE..=MAX_DRIVE).contains(&config.drive)
        {
            return Err(ModelDError::InvalidConfig);
        }
        Ok(Self {
            source_levels: config.source_levels,
            feedback_level: config.feedback_level,
            drive: config.drive,
            mode: config.mode,
        })
    }

    /// Mix one sample. The nonlinear mode compensates the cubic's small-signal
    /// gain and drive, while retaining its odd, saturated transfer characteristic.
    #[inline]
    pub fn sample(&mut self, sources: [f32; 3], feedback: f32) -> f32 {
        let input = sources[0] * self.source_levels[0]
            + sources[1] * self.source_levels[1]
            + sources[2] * self.source_levels[2]
            + feedback * self.feedback_level;
        match self.mode {
            MixerMode::Linear => input,
            MixerMode::Nonlinear => {
                let driven = (input * self.drive).clamp(-1.0, 1.0);
                let curved = driven * (CUBIC_LINEAR_GAIN - 0.5 * driven * driven);
                curved / (CUBIC_LINEAR_GAIN * self.drive)
            }
        }
    }

    pub const fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::{MixerConfig, MixerMode, ModelDMixer};

    fn config(mode: MixerMode) -> MixerConfig {
        MixerConfig {
            source_levels: [0.5, 0.25, 0.75],
            feedback_level: 0.5,
            drive: 2.0,
            mode,
        }
    }

    #[test]
    fn linear_mode_is_the_exact_weighted_sum() {
        let mut mixer = ModelDMixer::new(config(MixerMode::Linear)).unwrap();
        assert_eq!(mixer.sample([0.25, -0.5, 0.75], -0.25), 0.4375);
    }

    #[test]
    fn nonlinear_transfer_is_monotonic_within_declared_input_range() {
        let mut mixer = ModelDMixer::new(MixerConfig {
            source_levels: [1.0, 0.0, 0.0],
            feedback_level: 0.0,
            drive: 2.0,
            mode: MixerMode::Nonlinear,
        })
        .unwrap();
        let mut previous = -1.0;
        for step in 0..=200 {
            let input = -1.0 + step as f32 * 0.01;
            let output = mixer.sample([input, 0.0, 0.0], 0.0);
            assert!(
                output >= previous,
                "input={input}, previous={previous}, output={output}"
            );
            previous = output;
        }
    }

    #[test]
    fn nonlinear_mode_compresses_high_levels_and_adds_odd_harmonic_energy() {
        let mut nonlinear = ModelDMixer::new(MixerConfig {
            source_levels: [1.0, 0.0, 0.0],
            feedback_level: 0.0,
            drive: 2.0,
            mode: MixerMode::Nonlinear,
        })
        .unwrap();
        assert!(nonlinear.sample([0.9, 0.0, 0.0], 0.0).abs() < 0.9);

        let mut fundamental = 0.0_f32;
        let mut third = 0.0_f32;
        for index in 0..256 {
            let phase = core::f32::consts::TAU * index as f32 / 256.0;
            let output = nonlinear.sample([0.4 * phase.sin(), 0.0, 0.0], 0.0);
            fundamental += output * phase.sin();
            third += output * (3.0 * phase).sin();
        }
        assert!(fundamental.abs() > 1.0);
        assert!(third.abs() > 1.0e-3, "third={third}");
    }

    #[test]
    fn nonlinear_mode_is_finite_bounded_deterministic_resettable_and_allocation_free() {
        let config = config(MixerMode::Nonlinear);
        let mut first = ModelDMixer::new(config).unwrap();
        let mut second = ModelDMixer::new(config).unwrap();
        assert_no_alloc(|| {
            for index in 0..4_096 {
                let phase = core::f32::consts::TAU * index as f32 / 97.0;
                let sources = [phase.sin(), (2.0 * phase).sin(), (3.0 * phase).sin()];
                let feedback = (0.5 * phase).sin();
                let a = first.sample(sources, feedback);
                let b = second.sample(sources, feedback);
                assert_eq!(a, b);
                assert!(a.is_finite() && (-1.0..=1.0).contains(&a), "{a}");
            }
        });
        first.reset();
        let expected = first.sample([0.3, -0.2, 0.1], 0.25);
        first.sample([-0.8, 0.8, -0.8], -0.8);
        first.reset();
        assert_eq!(first.sample([0.3, -0.2, 0.1], 0.25), expected);
    }

    #[test]
    fn rejects_nonfinite_and_out_of_range_prepared_values() {
        for source_level in [-0.001, 1.001, f32::NAN, f32::INFINITY] {
            assert!(
                ModelDMixer::new(MixerConfig {
                    source_levels: [source_level, 0.5, 0.5],
                    ..config(MixerMode::Linear)
                })
                .is_err()
            );
        }
        for feedback_level in [-0.001, 1.001, f32::NAN, f32::INFINITY] {
            assert!(
                ModelDMixer::new(MixerConfig {
                    feedback_level,
                    ..config(MixerMode::Linear)
                })
                .is_err()
            );
        }
        for drive in [0.0, -0.001, 4.001, f32::NAN, f32::INFINITY] {
            assert!(
                ModelDMixer::new(MixerConfig {
                    drive,
                    ..config(MixerMode::Nonlinear)
                })
                .is_err()
            );
        }
    }
}
