use super::{
    ModelDError,
    contour::{ContourConfig, ModelDContour},
    ladder::{LadderConfig, LadderMode, ModelDLadder},
    mixer::{MixerConfig, MixerMode, ModelDMixer},
    vco::{ModelDVco, ModelDWaveform, VcoConfig},
};

/// Conservative bound for finite samples inside the isolated prepared voice.
///
/// Authored patches use substantially less than this bound. It is not a
/// clipping threshold and the sample path does not contain a limiter.
pub const MODEL_D_OUTPUT_BOUND: f32 = 4.0;

const MIN_MIXER_DRIVE: f32 = 1.0;
const MAX_MIXER_DRIVE: f32 = 4.0;
const MIN_LADDER_DRIVE: f32 = 1.0;
const MAX_LADDER_DRIVE: f32 = 4.0;
const CUTOFF_OCTAVE_NORMALIZED: f32 = 0.115_689_11;

/// Prepared controls for one authored monophonic Model D character voice.
#[derive(Clone, Copy, Debug)]
pub struct ModelDPatch {
    pub vcos: [VcoConfig; 3],
    pub source_levels: [f32; 3],
    pub feedback_level: f32,
    pub mixer_drive: f32,
    pub resonance: f32,
    pub ladder_drive: f32,
    pub filter_contour: ContourConfig,
    pub loudness_contour: ContourConfig,
    /// Base ladder cutoff in the ladder's normalized exponential domain.
    pub cutoff_normalized: f32,
    /// Cutoff movement per keyboard octave, from no tracking to full tracking.
    pub key_tracking: f32,
    /// Positive normalized cutoff movement contributed by the filter contour.
    pub filter_contour_amount: f32,
    pub output_gain: f32,
}

impl ModelDPatch {
    pub const fn bass() -> Self {
        Self {
            vcos: [
                VcoConfig {
                    semitone_offset: -12,
                    cents_offset: -1.1,
                    drift_cents: 0.8,
                    drift_hz: 0.13,
                    asymmetry: -0.10,
                    level_offset: 0.02,
                    reset_phase: 0.07,
                    waveform: ModelDWaveform::Saw,
                },
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: 0.8,
                    drift_cents: -1.1,
                    drift_hz: 0.17,
                    asymmetry: 0.08,
                    level_offset: -0.03,
                    reset_phase: 0.31,
                    waveform: ModelDWaveform::Rectangle,
                },
                VcoConfig {
                    semitone_offset: 12,
                    cents_offset: 1.7,
                    drift_cents: 1.3,
                    drift_hz: 0.21,
                    asymmetry: -0.06,
                    level_offset: 0.01,
                    reset_phase: 0.61,
                    waveform: ModelDWaveform::NarrowPulse,
                },
            ],
            source_levels: [0.88, 0.72, 0.34],
            feedback_level: 0.30,
            mixer_drive: 2.4,
            resonance: 0.34,
            ladder_drive: 2.2,
            filter_contour: ContourConfig {
                attack_seconds: 0.004,
                decay_seconds: 0.28,
                sustain_level: 0.24,
            },
            loudness_contour: ContourConfig {
                attack_seconds: 0.006,
                decay_seconds: 0.36,
                sustain_level: 0.70,
            },
            cutoff_normalized: 0.43,
            key_tracking: 0.55,
            filter_contour_amount: 0.26,
            output_gain: 0.42,
        }
    }

    pub const fn lead() -> Self {
        Self {
            vcos: [
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: -1.4,
                    drift_cents: 0.9,
                    drift_hz: 0.14,
                    asymmetry: -0.07,
                    level_offset: 0.02,
                    reset_phase: 0.11,
                    waveform: ModelDWaveform::Saw,
                },
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: 1.6,
                    drift_cents: -1.2,
                    drift_hz: 0.19,
                    asymmetry: 0.10,
                    level_offset: -0.01,
                    reset_phase: 0.43,
                    waveform: ModelDWaveform::WidePulse,
                },
                VcoConfig {
                    semitone_offset: 12,
                    cents_offset: 0.5,
                    drift_cents: 0.7,
                    drift_hz: 0.23,
                    asymmetry: 0.04,
                    level_offset: -0.04,
                    reset_phase: 0.73,
                    waveform: ModelDWaveform::Saw,
                },
            ],
            source_levels: [0.78, 0.66, 0.30],
            feedback_level: 0.20,
            mixer_drive: 2.1,
            resonance: 0.42,
            ladder_drive: 2.0,
            filter_contour: ContourConfig {
                attack_seconds: 0.018,
                decay_seconds: 0.32,
                sustain_level: 0.38,
            },
            loudness_contour: ContourConfig {
                attack_seconds: 0.012,
                decay_seconds: 0.42,
                sustain_level: 0.76,
            },
            cutoff_normalized: 0.56,
            key_tracking: 0.68,
            filter_contour_amount: 0.25,
            output_gain: 0.40,
        }
    }

    pub const fn filter_articulation() -> Self {
        Self {
            vcos: [
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: -0.9,
                    drift_cents: 0.7,
                    drift_hz: 0.12,
                    asymmetry: -0.05,
                    level_offset: 0.00,
                    reset_phase: 0.17,
                    waveform: ModelDWaveform::Triangle,
                },
                VcoConfig {
                    semitone_offset: 12,
                    cents_offset: 1.2,
                    drift_cents: -0.9,
                    drift_hz: 0.18,
                    asymmetry: 0.09,
                    level_offset: -0.02,
                    reset_phase: 0.47,
                    waveform: ModelDWaveform::Saw,
                },
                VcoConfig {
                    semitone_offset: 19,
                    cents_offset: -1.8,
                    drift_cents: 1.1,
                    drift_hz: 0.24,
                    asymmetry: -0.12,
                    level_offset: 0.03,
                    reset_phase: 0.79,
                    waveform: ModelDWaveform::NarrowPulse,
                },
            ],
            source_levels: [0.72, 0.58, 0.28],
            feedback_level: 0.14,
            mixer_drive: 2.0,
            resonance: 0.58,
            ladder_drive: 1.8,
            filter_contour: ContourConfig {
                attack_seconds: 0.002,
                decay_seconds: 0.18,
                sustain_level: 0.08,
            },
            loudness_contour: ContourConfig {
                attack_seconds: 0.004,
                decay_seconds: 0.30,
                sustain_level: 0.64,
            },
            cutoff_normalized: 0.35,
            key_tracking: 0.72,
            filter_contour_amount: 0.52,
            output_gain: 0.42,
        }
    }
}

/// Matched causal substitutions for the complete voice.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ModelDDiagnostics {
    pub linear_mixer: bool,
    pub linear_ladder: bool,
    pub idealize_oscillators: bool,
    pub disable_drift: bool,
    pub disable_feedback: bool,
}

impl ModelDDiagnostics {
    pub const fn full() -> Self {
        Self {
            linear_mixer: false,
            linear_ladder: false,
            idealize_oscillators: false,
            disable_drift: false,
            disable_feedback: false,
        }
    }

    pub const fn idealized_path() -> Self {
        Self {
            linear_mixer: true,
            linear_ladder: true,
            idealize_oscillators: true,
            disable_drift: true,
            disable_feedback: true,
        }
    }

    pub const fn linear_mixer() -> Self {
        Self {
            linear_mixer: true,
            ..Self::full()
        }
    }

    pub const fn linear_ladder() -> Self {
        Self {
            linear_ladder: true,
            ..Self::full()
        }
    }

    pub const fn no_drift_or_feedback() -> Self {
        Self {
            disable_drift: true,
            disable_feedback: true,
            ..Self::full()
        }
    }
}

/// Prepared monophonic circuit-informed character voice.
///
/// Construction and note changes prepare every nontrivial coefficient. The
/// sample path performs fixed scalar work without allocation, I/O, locking, or
/// per-sample transcendental setup.
#[derive(Clone, Debug)]
pub struct ModelDVoice {
    vcos: [ModelDVco; 3],
    mixer: ModelDMixer,
    ladder: ModelDLadder,
    filter_contour: ModelDContour,
    loudness_contour: ModelDContour,
    cutoff_normalized: f32,
    note_cutoff_offset: f32,
    key_tracking: f32,
    filter_contour_amount: f32,
    output_gain: f32,
    velocity: f32,
    feedback_sample: f32,
}

impl ModelDVoice {
    pub fn new(
        sample_rate: f32,
        patch: ModelDPatch,
        diagnostics: ModelDDiagnostics,
    ) -> Result<Self, ModelDError> {
        validate_patch(patch)?;

        let vco_configs = patch.vcos.map(|config| {
            if diagnostics.idealize_oscillators {
                VcoConfig {
                    cents_offset: 0.0,
                    drift_cents: 0.0,
                    asymmetry: 0.0,
                    level_offset: 0.0,
                    ..config
                }
            } else if diagnostics.disable_drift {
                VcoConfig {
                    drift_cents: 0.0,
                    ..config
                }
            } else {
                config
            }
        });
        let [vco_one, vco_two, vco_three] = vco_configs;
        let vcos = [
            ModelDVco::new(sample_rate, vco_one)?,
            ModelDVco::new(sample_rate, vco_two)?,
            ModelDVco::new(sample_rate, vco_three)?,
        ];
        let mixer = ModelDMixer::new(MixerConfig {
            source_levels: patch.source_levels,
            feedback_level: if diagnostics.disable_feedback {
                0.0
            } else {
                patch.feedback_level
            },
            drive: patch.mixer_drive,
            mode: if diagnostics.linear_mixer {
                MixerMode::Linear
            } else {
                MixerMode::Nonlinear
            },
        })?;
        let ladder = ModelDLadder::new(
            sample_rate,
            LadderConfig {
                resonance: patch.resonance,
                drive: patch.ladder_drive,
                mode: if diagnostics.linear_ladder {
                    LadderMode::Linear
                } else {
                    LadderMode::Nonlinear
                },
            },
        )?;

        Ok(Self {
            vcos,
            mixer,
            ladder,
            filter_contour: ModelDContour::new(sample_rate, patch.filter_contour)?,
            loudness_contour: ModelDContour::new(sample_rate, patch.loudness_contour)?,
            cutoff_normalized: patch.cutoff_normalized,
            note_cutoff_offset: 0.0,
            key_tracking: patch.key_tracking,
            filter_contour_amount: patch.filter_contour_amount,
            output_gain: patch.output_gain,
            velocity: 0.0,
            feedback_sample: 0.0,
        })
    }

    /// Start or retrigger the monophonic note. Velocity is bounded to 0–1;
    /// non-finite or non-positive values leave the voice safely idle.
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        if !velocity.is_finite() || velocity <= 0.0 {
            self.reset();
            return;
        }

        for vco in &mut self.vcos {
            vco.set_note(note);
        }
        self.note_cutoff_offset =
            (f32::from(note) - 60.0) * (CUTOFF_OCTAVE_NORMALIZED / 12.0) * self.key_tracking;
        self.velocity = velocity.clamp(0.0, 1.0);
        self.filter_contour.note_on();
        self.loudness_contour.note_on();
    }

    pub fn note_off(&mut self) {
        self.filter_contour.note_off();
        self.loudness_contour.note_off();
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        if self.loudness_contour.is_idle() {
            self.feedback_sample = 0.0;
            return 0.0;
        }

        let sources = [
            self.vcos[0].sample(),
            self.vcos[1].sample(),
            self.vcos[2].sample(),
        ];
        let mixed = self.mixer.sample(sources, self.feedback_sample);
        let filter_level = self.filter_contour.advance();
        let cutoff = (self.cutoff_normalized
            + self.note_cutoff_offset
            + self.filter_contour_amount * filter_level)
            .clamp(0.0, 1.0);
        let filtered = self.ladder.sample(mixed, cutoff);
        let loudness = self.loudness_contour.advance();
        let output = filtered * loudness * self.velocity * self.output_gain;

        if loudness == 0.0 {
            self.finish_idle();
            return 0.0;
        }
        if output.is_finite() && output.abs() < MODEL_D_OUTPUT_BOUND {
            self.feedback_sample = output;
            output
        } else {
            self.reset();
            0.0
        }
    }

    pub fn is_idle(&self) -> bool {
        self.loudness_contour.is_idle()
    }

    pub fn reset(&mut self) {
        for vco in &mut self.vcos {
            vco.reset();
        }
        self.mixer.reset();
        self.ladder.reset();
        self.filter_contour.reset();
        self.loudness_contour.reset();
        self.note_cutoff_offset = 0.0;
        self.velocity = 0.0;
        self.feedback_sample = 0.0;
    }

    fn finish_idle(&mut self) {
        for vco in &mut self.vcos {
            vco.reset();
        }
        self.mixer.reset();
        self.ladder.reset();
        self.filter_contour.reset();
        self.feedback_sample = 0.0;
        self.velocity = 0.0;
    }
}

fn validate_patch(patch: ModelDPatch) -> Result<(), ModelDError> {
    let source_levels_are_valid = patch
        .source_levels
        .iter()
        .all(|level| level.is_finite() && (0.0..=1.0).contains(level));
    if !source_levels_are_valid
        || !finite_in_range(patch.feedback_level, 0.0, 1.0)
        || !finite_in_range(patch.mixer_drive, MIN_MIXER_DRIVE, MAX_MIXER_DRIVE)
        || !finite_in_range(patch.resonance, 0.0, 1.0)
        || !finite_in_range(patch.ladder_drive, MIN_LADDER_DRIVE, MAX_LADDER_DRIVE)
        || !finite_in_range(patch.cutoff_normalized, 0.0, 1.0)
        || !finite_in_range(patch.key_tracking, 0.0, 1.0)
        || !finite_in_range(patch.filter_contour_amount, 0.0, 1.0)
        || !finite_in_range(patch.output_gain, 0.0, 1.0)
    {
        return Err(ModelDError::InvalidConfig);
    }
    Ok(())
}

fn finite_in_range(value: f32, minimum: f32, maximum: f32) -> bool {
    value.is_finite() && (minimum..=maximum).contains(&value)
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::{MODEL_D_OUTPUT_BOUND, ModelDDiagnostics, ModelDPatch, ModelDVoice};

    const SAMPLE_RATE: f32 = 48_000.0;
    const SUSTAIN_SAMPLES: usize = 12_000;
    const MAX_RELEASE_SAMPLES: usize = 48_000;

    fn render_bass(diagnostics: ModelDDiagnostics) -> Vec<f32> {
        let mut voice = ModelDVoice::new(SAMPLE_RATE, ModelDPatch::bass(), diagnostics).unwrap();
        voice.note_on(48, 0.82);
        let mut samples = Vec::with_capacity(SUSTAIN_SAMPLES + MAX_RELEASE_SAMPLES + 1);
        for _ in 0..SUSTAIN_SAMPLES {
            samples.push(voice.sample());
        }
        voice.note_off();
        for _ in 0..MAX_RELEASE_SAMPLES {
            samples.push(voice.sample());
            if voice.is_idle() {
                break;
            }
        }
        samples.push(voice.sample());
        samples
    }

    fn active_duration(samples: &[f32]) -> usize {
        samples
            .iter()
            .rposition(|sample| *sample != 0.0)
            .map_or(0, |index| index + 1)
    }

    fn bass_pitch_period(samples: &[f32]) -> usize {
        (650..=820)
            .max_by(|left, right| {
                let correlation = |lag: usize| {
                    samples[lag..]
                        .iter()
                        .zip(samples)
                        .map(|(current, delayed)| f64::from(*current) * f64::from(*delayed))
                        .sum::<f64>()
                };
                correlation(*left)
                    .partial_cmp(&correlation(*right))
                    .unwrap()
            })
            .unwrap()
    }

    #[test]
    fn authored_patches_prepare_as_complete_voices() {
        for patch in [
            ModelDPatch::bass(),
            ModelDPatch::lead(),
            ModelDPatch::filter_articulation(),
        ] {
            let mut voice =
                ModelDVoice::new(SAMPLE_RATE, patch, ModelDDiagnostics::full()).unwrap();
            assert!(voice.is_idle());
            assert_eq!(voice.sample(), 0.0);
            voice.note_on(60, 1.0);
            assert!(!voice.is_idle());
            assert!((0..2_048).any(|_| voice.sample() != 0.0));
        }
    }

    #[test]
    fn bass_note_lifecycle_is_finite_bounded_and_returns_to_exact_idle_zero() {
        let samples = render_bass(ModelDDiagnostics::full());
        let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(peak > 0.001, "peak={peak}");
        assert!(peak < MODEL_D_OUTPUT_BOUND, "peak={peak}");
        assert_eq!(samples.last(), Some(&0.0));
        assert!(active_duration(&samples) < samples.len());
    }

    #[test]
    fn reset_replays_the_same_note_bit_for_bit() {
        let mut voice =
            ModelDVoice::new(SAMPLE_RATE, ModelDPatch::bass(), ModelDDiagnostics::full()).unwrap();
        let render = |voice: &mut ModelDVoice| {
            voice.note_on(43, 0.67);
            let sustain: Vec<_> = (0..4_096).map(|_| voice.sample()).collect();
            voice.note_off();
            let release: Vec<_> = (0..2_048).map(|_| voice.sample()).collect();
            (sustain, release)
        };

        let expected = render(&mut voice);
        voice.reset();
        assert!(voice.is_idle());
        assert_eq!(voice.sample(), 0.0);
        assert_eq!(render(&mut voice), expected);
    }

    #[test]
    fn prepared_sample_path_is_allocation_free() {
        let mut voice =
            ModelDVoice::new(SAMPLE_RATE, ModelDPatch::bass(), ModelDDiagnostics::full()).unwrap();
        voice.note_on(48, 0.75);
        assert_no_alloc(|| {
            for _ in 0..8_192 {
                let sample = voice.sample();
                assert!(sample.is_finite());
            }
        });
    }

    #[test]
    fn matched_ablation_modes_change_character_but_not_duration_or_pitch() {
        let full = render_bass(ModelDDiagnostics::full());
        let ablations = [
            render_bass(ModelDDiagnostics::linear_mixer()),
            render_bass(ModelDDiagnostics::linear_ladder()),
            render_bass(ModelDDiagnostics::no_drift_or_feedback()),
        ];
        let full_duration = active_duration(&full);
        let pitch_region = 4_000..12_000;
        let full_period = bass_pitch_period(&full[pitch_region.clone()]);

        for ablation in ablations {
            assert_eq!(ablation.len(), full.len());
            assert_eq!(active_duration(&ablation), full_duration);
            let residual = full
                .iter()
                .zip(&ablation)
                .map(|(full, ablated)| {
                    let difference = f64::from(*full) - f64::from(*ablated);
                    difference * difference
                })
                .sum::<f64>()
                .sqrt();
            assert!(residual > 1.0e-5, "residual={residual}");

            let period = bass_pitch_period(&ablation[pitch_region.clone()]);
            assert!(
                period.abs_diff(full_period) <= 2,
                "full_period={full_period}, ablation_period={period}"
            );
        }
    }

    #[test]
    fn invalid_sample_rates_patch_values_and_velocity_are_bounded_safely() {
        assert!(ModelDVoice::new(0.0, ModelDPatch::bass(), ModelDDiagnostics::full()).is_err());
        assert!(
            ModelDVoice::new(f32::NAN, ModelDPatch::bass(), ModelDDiagnostics::full()).is_err()
        );

        let mut invalid_patch = ModelDPatch::bass();
        invalid_patch.output_gain = f32::NAN;
        assert!(ModelDVoice::new(SAMPLE_RATE, invalid_patch, ModelDDiagnostics::full()).is_err());

        let mut invalid_patch = ModelDPatch::bass();
        invalid_patch.filter_contour_amount = 1.01;
        assert!(ModelDVoice::new(SAMPLE_RATE, invalid_patch, ModelDDiagnostics::full()).is_err());

        let mut voice =
            ModelDVoice::new(SAMPLE_RATE, ModelDPatch::bass(), ModelDDiagnostics::full()).unwrap();
        voice.note_on(48, f32::NAN);
        assert!(voice.is_idle());
        assert_eq!(voice.sample(), 0.0);
        voice.note_on(48, 10.0);
        assert!((0..2_048).all(|_| voice.sample().abs() < MODEL_D_OUTPUT_BOUND));
    }
}
