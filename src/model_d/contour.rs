use super::ModelDError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContourConfig {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContourStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

fn prepared_sample_count(seconds: f32, sample_rate: f32) -> Result<u32, ModelDError> {
    let samples = f64::from(seconds) * f64::from(sample_rate);
    if !samples.is_finite() || samples < 1.0 || samples >= f64::from(u32::MAX) {
        return Err(ModelDError::InvalidConfig);
    }

    let rounded = samples.round();
    if rounded < 1.0 || rounded >= f64::from(u32::MAX) {
        return Err(ModelDError::InvalidConfig);
    }
    Ok(rounded as u32)
}

/// A fixed-state Model D style ADS contour with the decay time reused for release.
#[derive(Clone, Copy, Debug)]
pub struct ModelDContour {
    config: ContourConfig,
    stage: ContourStage,
    level: f32,
    attack_step: f32,
    decay_step: f32,
    attack_samples: u32,
    decay_samples: u32,
    attack_remaining: u32,
    decay_remaining: u32,
    release_step: f32,
    release_remaining: u32,
}

impl ModelDContour {
    pub fn new(sample_rate: f32, config: ContourConfig) -> Result<Self, ModelDError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(ModelDError::InvalidSampleRate);
        }
        if !config.attack_seconds.is_finite()
            || config.attack_seconds <= 0.0
            || !config.decay_seconds.is_finite()
            || config.decay_seconds <= 0.0
            || !config.sustain_level.is_finite()
            || !(0.0..=1.0).contains(&config.sustain_level)
        {
            return Err(ModelDError::InvalidConfig);
        }
        let attack_samples = prepared_sample_count(config.attack_seconds, sample_rate)?;
        let decay_samples = prepared_sample_count(config.decay_seconds, sample_rate)?;

        Ok(Self {
            config,
            stage: ContourStage::Idle,
            level: 0.0,
            attack_step: 1.0 / attack_samples as f32,
            decay_step: (1.0 - config.sustain_level) / decay_samples as f32,
            attack_samples,
            decay_samples,
            attack_remaining: 0,
            decay_remaining: 0,
            release_step: 0.0,
            release_remaining: 0,
        })
    }

    pub fn note_on(&mut self) {
        self.attack_remaining = self.attack_samples;
        self.release_step = 0.0;
        self.release_remaining = 0;
        self.stage = ContourStage::Attack;
    }

    pub fn restart(&mut self) {
        self.level = 0.0;
        self.attack_remaining = self.attack_samples;
        self.decay_remaining = 0;
        self.release_step = 0.0;
        self.release_remaining = 0;
        self.stage = ContourStage::Attack;
    }

    pub fn note_off(&mut self) {
        if self.stage != ContourStage::Idle {
            if self.level <= 0.0 {
                self.reset();
                return;
            }
            self.release_step = self.level / self.decay_samples as f32;
            self.release_remaining = self.decay_samples;
            self.stage = ContourStage::Release;
        }
    }

    #[inline]
    pub fn advance(&mut self) -> f32 {
        match self.stage {
            ContourStage::Idle => self.level = 0.0,
            ContourStage::Attack => {
                if self.attack_remaining <= 1 {
                    self.level = 1.0;
                    self.attack_remaining = 0;
                    self.decay_remaining = self.decay_samples;
                    self.stage = ContourStage::Decay;
                } else {
                    self.level += self.attack_step;
                    self.attack_remaining -= 1;
                }
            }
            ContourStage::Decay => {
                if self.decay_remaining <= 1 {
                    self.level = self.config.sustain_level;
                    self.decay_remaining = 0;
                    self.stage = ContourStage::Sustain;
                } else {
                    self.level -= self.decay_step;
                    self.decay_remaining -= 1;
                }
            }
            ContourStage::Sustain => self.level = self.config.sustain_level,
            ContourStage::Release => {
                if self.release_remaining <= 1 {
                    self.level = 0.0;
                    self.release_remaining = 0;
                    self.stage = ContourStage::Idle;
                } else {
                    self.level -= self.release_step;
                    self.release_remaining -= 1;
                }
            }
        }
        self.level
    }

    pub const fn level(&self) -> f32 {
        self.level
    }

    pub fn is_idle(&self) -> bool {
        self.stage == ContourStage::Idle
    }

    pub fn reset(&mut self) {
        self.level = 0.0;
        self.attack_remaining = 0;
        self.decay_remaining = 0;
        self.release_step = 0.0;
        self.release_remaining = 0;
        self.stage = ContourStage::Idle;
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::{ContourConfig, ModelDContour};

    fn config() -> ContourConfig {
        ContourConfig {
            attack_seconds: 0.002,
            decay_seconds: 0.003,
            sustain_level: 0.4,
        }
    }

    #[test]
    fn contour_traverses_attack_decay_sustain_and_decay_length_release() {
        let mut contour = ModelDContour::new(1_000.0, config()).unwrap();
        assert_eq!(contour.advance(), 0.0);
        assert!(contour.is_idle());

        contour.note_on();
        assert_eq!(contour.advance(), 0.5);
        assert_eq!(contour.advance(), 1.0);
        assert_eq!(contour.advance(), 0.8);
        assert_eq!(contour.advance(), 0.6);
        assert!((contour.advance() - 0.4).abs() < 1.0e-6);

        contour.note_off();
        assert!(!contour.is_idle());
        assert!((contour.advance() - (0.4 * 2.0 / 3.0)).abs() < 1.0e-6);
        assert!((contour.advance() - (0.4 / 3.0)).abs() < 1.0e-6);
        assert_eq!(contour.advance(), 0.0);
        assert!(contour.is_idle());
        assert_eq!(contour.level(), 0.0);
    }

    #[test]
    fn exact_sample_counts_complete_attack_decay_and_release_on_the_final_sample() {
        let mut contour = ModelDContour::new(
            1_000.0,
            ContourConfig {
                attack_seconds: 0.012,
                decay_seconds: 0.003,
                sustain_level: 0.4,
            },
        )
        .unwrap();
        contour.note_on();

        for _ in 0..11 {
            assert!(contour.advance() < 1.0);
        }
        assert_eq!(contour.advance(), 1.0);

        for _ in 0..2 {
            assert!(contour.advance() > 0.4);
        }
        assert_eq!(contour.advance(), 0.4);

        contour.note_off();
        for _ in 0..2 {
            assert!(contour.advance() > 0.0);
        }
        assert_eq!(contour.advance(), 0.0);
        assert!(contour.is_idle());
    }

    #[test]
    fn restart_is_deterministic_and_reset_is_exactly_idle() {
        let mut contour = ModelDContour::new(1_000.0, config()).unwrap();
        contour.note_on();
        contour.advance();
        contour.note_off();
        contour.restart();
        let expected = [contour.advance(), contour.advance(), contour.advance()];

        contour.advance();
        contour.reset();
        assert!(contour.is_idle());
        assert_eq!(contour.level(), 0.0);
        contour.restart();
        assert_eq!(
            [contour.advance(), contour.advance(), contour.advance()],
            expected
        );
    }

    #[test]
    fn rejects_invalid_prepared_configuration() {
        for attack_seconds in [0.0, -0.001, f32::NAN, f32::INFINITY] {
            assert!(
                ModelDContour::new(
                    48_000.0,
                    ContourConfig {
                        attack_seconds,
                        ..config()
                    }
                )
                .is_err()
            );
        }
        for decay_seconds in [0.0, -0.001, f32::NAN, f32::INFINITY] {
            assert!(
                ModelDContour::new(
                    48_000.0,
                    ContourConfig {
                        decay_seconds,
                        ..config()
                    }
                )
                .is_err()
            );
        }
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    sustain_level: 1.001,
                    ..config()
                }
            )
            .is_err()
        );
        assert!(ModelDContour::new(f32::NAN, config()).is_err());
    }

    #[test]
    fn rejects_overflowing_zero_and_unrepresentable_derived_sample_counts() {
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    attack_seconds: f32::MAX,
                    ..config()
                },
            )
            .is_err()
        );
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    decay_seconds: f32::MAX,
                    ..config()
                },
            )
            .is_err()
        );
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    attack_seconds: f32::MIN_POSITIVE,
                    ..config()
                },
            )
            .is_err()
        );
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    decay_seconds: f32::MIN_POSITIVE,
                    ..config()
                },
            )
            .is_err()
        );
        assert!(
            ModelDContour::new(
                48_000.0,
                ContourConfig {
                    attack_seconds: 100_000.0,
                    ..config()
                },
            )
            .is_err()
        );
    }

    #[test]
    fn advance_is_allocation_free() {
        let mut contour = ModelDContour::new(48_000.0, config()).unwrap();
        contour.note_on();
        assert_no_alloc(|| {
            for _ in 0..4_096 {
                let level = contour.advance();
                assert!(level.is_finite() && (0.0..=1.0).contains(&level));
            }
            contour.note_off();
            for _ in 0..4_096 {
                let level = contour.advance();
                assert!(level.is_finite() && (0.0..=1.0).contains(&level));
            }
        });
        assert!(contour.is_idle());
        assert_eq!(contour.level(), 0.0);
    }
}
