use crate::model_d::{
    ModelDError,
    ladder::{LadderConfig, LadderMode, ModelDLadder},
    vco::{ModelDVco, ModelDWaveform, VcoConfig},
    voice::{ModelDDiagnostics, ModelDPatch, ModelDVoice},
};

const CANONICAL_SAMPLE_RATE: u32 = 48_000;
const MIN_RENDER_SAMPLE_RATE: u32 = 8_000;
const MAX_RENDER_SAMPLE_RATE: u32 = 384_000;
const BASS_PRESENTATION_GAIN: f32 = 1.0;
const LEAD_PRESENTATION_GAIN: f32 = 0.75;
const FILTER_PRESENTATION_GAIN: f32 = 1.6;
const BASS_THIRD_OSCILLATOR_LEVEL: f32 = 0.14;
const ANALYSIS_RATE_FACTOR: usize = 4;
pub const MODEL_D_ALIAS_ANALYSIS_FILTER_TAPS: usize = 257;
pub const MODEL_D_ALIAS_ANALYSIS_FILTER_CUTOFF_HZ: f64 = 21_600.0;
const ALIAS_WARMUP_FRAMES_48K: usize = 48_000;
const ALIAS_MEASUREMENT_FRAMES: usize = 131_072;
pub const MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS: usize = 4;
pub const MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB: f64 = 6.0;
pub const MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_POWER_RATIO: f64 = 3.981_071_705_534_972_2;
pub const MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB: f64 = -24.0;
pub const MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB: f64 = -6.0;
pub const MODEL_D_FULL_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB: f64 = -24.0;
pub const MODEL_D_FULL_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB: f64 = -2.0;
pub const MODEL_D_FILTER_CUTOFF_ERROR_MAX: f32 = 0.08;
pub const MODEL_D_FILTER_SLOPE_MIN_DB_PER_OCTAVE: f32 = 20.0;
pub const MODEL_D_FILTER_SLOPE_MAX_DB_PER_OCTAVE: f32 = 28.0;
pub const MODEL_D_FILTER_RESONANCE_RATIO_MIN: f32 = 1.05;

/// The ladder's 63-tap decimator contributes 31 samples of delay at its
/// four-times internal rate, or 7.75 samples at its host rate.
pub const MODEL_D_LADDER_GROUP_DELAY_INTERNAL: f64 = 31.0;
pub const MODEL_D_LADDER_GROUP_DELAY_HOST: f64 =
    MODEL_D_LADDER_GROUP_DELAY_INTERNAL / ANALYSIS_RATE_FACTOR as f64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScoreAction {
    NoteOn { note: u8, velocity_milli: u16 },
    NoteOff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreEvent {
    pub frame_48k: usize,
    pub action: ScoreAction,
}

const BASS_EVENTS: [ScoreEvent; 8] = [
    note_on(0, 36, 880),
    note_off(28_800),
    note_on(38_400, 43, 820),
    note_off(72_000),
    note_on(81_600, 36, 900),
    note_off(120_000),
    note_on(129_600, 48, 780),
    note_off(177_600),
];
const BASS_DURATION_FRAMES: usize = 264_000;

const LEAD_EVENTS: [ScoreEvent; 10] = [
    note_on(0, 60, 820),
    note_off(24_000),
    note_on(28_800, 64, 780),
    note_off(52_800),
    note_on(57_600, 67, 840),
    note_off(81_600),
    note_on(86_400, 72, 760),
    note_off(120_000),
    note_on(124_800, 69, 800),
    note_off(158_400),
];
const LEAD_DURATION_FRAMES: usize = 216_000;

const FILTER_EVENTS: [ScoreEvent; 10] = [
    note_on(0, 48, 900),
    note_off(14_400),
    note_on(19_200, 55, 860),
    note_off(33_600),
    note_on(38_400, 60, 900),
    note_off(52_800),
    note_on(57_600, 67, 820),
    note_off(72_000),
    note_on(76_800, 60, 880),
    note_off(96_000),
];
const FILTER_DURATION_FRAMES: usize = 158_400;

const fn note_on(frame_48k: usize, note: u8, velocity_milli: u16) -> ScoreEvent {
    ScoreEvent {
        frame_48k,
        action: ScoreAction::NoteOn {
            note,
            velocity_milli,
        },
    }
}

const fn note_off(frame_48k: usize) -> ScoreEvent {
    ScoreEvent {
        frame_48k,
        action: ScoreAction::NoteOff,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditionKind {
    FullBassPhrase,
    FullLeadPhrase,
    FullFilterArticulation,
    MatchedIdealizedPath,
    MatchedLinearMixer,
    MatchedLinearLadder,
    MatchedNoDriftOrFeedback,
}

impl AuditionKind {
    pub const ALL: [Self; 7] = [
        Self::FullBassPhrase,
        Self::FullLeadPhrase,
        Self::FullFilterArticulation,
        Self::MatchedIdealizedPath,
        Self::MatchedLinearMixer,
        Self::MatchedLinearLadder,
        Self::MatchedNoDriftOrFeedback,
    ];

    pub const fn filename(self) -> &'static str {
        match self {
            Self::FullBassPhrase => "01_full_bass_phrase.wav",
            Self::FullLeadPhrase => "02_full_lead_phrase.wav",
            Self::FullFilterArticulation => "03_full_filter_articulation.wav",
            Self::MatchedIdealizedPath => "04_matched_idealized_path.wav",
            Self::MatchedLinearMixer => "05_matched_linear_mixer.wav",
            Self::MatchedLinearLadder => "06_matched_linear_ladder.wav",
            Self::MatchedNoDriftOrFeedback => "07_matched_no_drift_or_feedback.wav",
        }
    }

    pub const fn score_name(self) -> &'static str {
        match self {
            Self::FullLeadPhrase => "lead-phrase",
            Self::FullFilterArticulation => "filter-articulation",
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => "bass-phrase",
        }
    }

    pub const fn presentation_gain(self) -> f32 {
        match self {
            Self::FullLeadPhrase => LEAD_PRESENTATION_GAIN,
            Self::FullFilterArticulation => FILTER_PRESENTATION_GAIN,
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => BASS_PRESENTATION_GAIN,
        }
    }

    pub const fn score_events(self) -> &'static [ScoreEvent] {
        match self {
            Self::FullLeadPhrase => &LEAD_EVENTS,
            Self::FullFilterArticulation => &FILTER_EVENTS,
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => &BASS_EVENTS,
        }
    }

    pub const fn duration_frames_48k(self) -> usize {
        match self {
            Self::FullLeadPhrase => LEAD_DURATION_FRAMES,
            Self::FullFilterArticulation => FILTER_DURATION_FRAMES,
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => BASS_DURATION_FRAMES,
        }
    }

    const fn patch(self) -> ModelDPatch {
        match self {
            Self::FullLeadPhrase => ModelDPatch::lead(),
            Self::FullFilterArticulation => ModelDPatch::filter_articulation(),
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => ModelDPatch::bass(),
        }
    }

    const fn diagnostics(self) -> ModelDDiagnostics {
        match self {
            Self::FullBassPhrase | Self::FullLeadPhrase | Self::FullFilterArticulation => {
                ModelDDiagnostics::full()
            }
            Self::MatchedIdealizedPath => ModelDDiagnostics::idealized_path(),
            Self::MatchedLinearMixer => ModelDDiagnostics::linear_mixer(),
            Self::MatchedLinearLadder => ModelDDiagnostics::linear_ladder(),
            Self::MatchedNoDriftOrFeedback => ModelDDiagnostics::no_drift_or_feedback(),
        }
    }

    const fn expected_pitch_note(self) -> u8 {
        match self {
            Self::FullBassPhrase
            | Self::MatchedIdealizedPath
            | Self::MatchedLinearMixer
            | Self::MatchedLinearLadder
            | Self::MatchedNoDriftOrFeedback => 24,
            Self::FullLeadPhrase => 60,
            Self::FullFilterArticulation => 48,
        }
    }

    const fn is_ablation(self) -> bool {
        matches!(
            self,
            Self::MatchedIdealizedPath
                | Self::MatchedLinearMixer
                | Self::MatchedLinearLadder
                | Self::MatchedNoDriftOrFeedback
        )
    }

    const fn uses_bass_score(self) -> bool {
        !matches!(self, Self::FullLeadPhrase | Self::FullFilterArticulation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelDMetrics {
    pub peak: f64,
    pub rms: f64,
    pub dc: f64,
    pub maximum_jump: f64,
    pub headroom_db: f64,
    pub pitch_hz: f64,
    pub pitch_error_cents: f64,
    pub presentation_gain: f32,
    pub zero_tail_frames: usize,
    pub ablation_residual_rms: f64,
    pub sample_hash: u64,
    pub score_hash: u64,
    pub finite: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelDRender {
    pub samples: Vec<f32>,
    pub metrics: ModelDMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelDAliasEvidence {
    pub configuration: ModelDAliasProbeConfiguration,
    pub proxy_method: &'static str,
    pub source_levels: [f32; 3],
    pub mixer_drive: f32,
    pub ladder_drive: f32,
    pub static_oscillator_imperfections_enabled: bool,
    pub feedback_enabled: bool,
    pub drift_frozen_for_stationary_analysis: bool,
    /// Nonharmonic 48 kHz out-of-mask energy relative to target signal energy.
    /// This proxy deliberately excludes energy on or near legitimate harmonic
    /// bins, whose coverage and half-width are reported separately.
    pub nonharmonic_foldback_proxy_db: f64,
    /// Native 192 kHz floor for the same nonharmonic out-of-mask proxy and
    /// physical 0–24 kHz band.
    pub reference_nonharmonic_floor_db: f64,
    /// Positive nonharmonic proxy power above the independently measured floor.
    /// `None` means that the estimate is floor-limited, so no excess can be
    /// resolved without inventing a number below the reference floor.
    pub excess_nonharmonic_foldback_proxy_db: Option<f64>,
    pub nonharmonic_proxy_floor_limited: bool,
    /// Raw time residual after fixed sinc resampling and the declared 7.75-host
    /// sample ladder-FIR alignment. It deliberately remains a non-acceptance
    /// diagnostic because it conflates transfer and phase with alias energy.
    pub transfer_conflated_residual_db: f64,
    pub target_fundamental_hz: f64,
    pub reference_fundamental_hz: f64,
    pub resolution_hz: f64,
    pub harmonic_mask_half_width_bins: usize,
    pub harmonic_mask_coverage: f64,
    /// Gain- and phase-invariant overtone-magnitude difference between the
    /// nonlinear probe and its matched linear path, relative to the nonlinear
    /// path's total harmonic energy. DC and the fundamental are excluded from
    /// the difference numerator.
    pub nonlinear_overtone_magnitude_difference_db: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDAliasProbeConfiguration {
    ControlledDiagnostic,
    FullAuthoredBassStatic,
}

impl ModelDAliasProbeConfiguration {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ControlledDiagnostic => "controlled_diagnostic",
            Self::FullAuthoredBassStatic => "full_authored_bass_static_drift_frozen",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelDVcoPitchEvidence {
    pub note: u8,
    pub sample_rate: u32,
    pub configured_static_cents: f32,
    pub configured_drift_cents: f32,
    pub mean_pitch_hz: f64,
    pub mean_pitch_error_cents: f64,
    pub minimum_drift_cents: f64,
    pub maximum_drift_cents: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelDVcoAliasEvidence {
    pub waveform: ModelDWaveform,
    pub note: u8,
    pub sample_rate: u32,
    pub asymmetry: f32,
    pub pulse_width: f32,
    pub nonharmonic_foldback_proxy_db: f64,
    pub acceptance_bound_db: f64,
}

pub fn measure_vco_pitch_matrix() -> Result<Vec<ModelDVcoPitchEvidence>, ModelDError> {
    const STATIC_CENTS: f32 = -2.0;
    const DRIFT_CENTS: f32 = 1.5;
    const DRIFT_HZ: f32 = 0.5;
    let mut rows = Vec::with_capacity(9);
    for sample_rate in [44_100_u32, 48_000, 96_000] {
        for note in [36_u8, 60, 84] {
            let mut vco = ModelDVco::new(
                sample_rate as f32,
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: STATIC_CENTS,
                    drift_cents: DRIFT_CENTS,
                    drift_hz: DRIFT_HZ,
                    asymmetry: 0.37,
                    level_offset: 0.02,
                    reset_phase: 0.25,
                    waveform: ModelDWaveform::Triangle,
                },
            )?;
            vco.set_note(note);
            let expected_hz =
                midi_frequency(note) * 2.0_f64.powf(f64::from(STATIC_CENTS) / 1_200.0);
            let expected_increment = f64::from(vco.prepared_static_phase_increment());
            let frames = (2 * sample_rate) as usize;
            let mut increment_sum = 0.0_f64;
            let mut minimum_drift_cents = f64::INFINITY;
            let mut maximum_drift_cents = f64::NEG_INFINITY;
            for _ in 0..frames {
                let increment = f64::from(vco.prepared_phase_increment());
                increment_sum += increment;
                let drift_cents = 1_200.0 * (increment / expected_increment).log2();
                minimum_drift_cents = minimum_drift_cents.min(drift_cents);
                maximum_drift_cents = maximum_drift_cents.max(drift_cents);
                vco.sample();
            }
            let mean_pitch_hz = increment_sum / frames as f64 * f64::from(sample_rate);
            rows.push(ModelDVcoPitchEvidence {
                note,
                sample_rate,
                configured_static_cents: STATIC_CENTS,
                configured_drift_cents: DRIFT_CENTS,
                mean_pitch_hz,
                mean_pitch_error_cents: 1_200.0 * (mean_pitch_hz / expected_hz).log2(),
                minimum_drift_cents,
                maximum_drift_cents,
            });
        }
    }
    Ok(rows)
}

pub fn measure_vco_waveform_alias_matrix() -> Result<Vec<ModelDVcoAliasEvidence>, ModelDError> {
    const NOTE: u8 = 96;
    const FRAMES: usize = 131_072;
    let configurations = [
        (ModelDWaveform::Triangle, 1.0, 0.50, -40.0),
        (ModelDWaveform::Saw, 1.0, 0.50, -28.0),
        (ModelDWaveform::Rectangle, 0.0, 0.50, -28.0),
        (ModelDWaveform::WidePulse, 1.0, 0.90, -24.0),
        (ModelDWaveform::NarrowPulse, -1.0, 0.10, -24.0),
    ];
    let mut rows = Vec::with_capacity(15);
    for sample_rate in [44_100_u32, 48_000, 96_000] {
        for (waveform, asymmetry, pulse_width, acceptance_bound_db) in configurations {
            let mut vco = ModelDVco::new(
                sample_rate as f32,
                VcoConfig {
                    semitone_offset: 0,
                    cents_offset: 0.0,
                    drift_cents: 0.0,
                    drift_hz: 0.25,
                    asymmetry,
                    level_offset: 0.0,
                    reset_phase: 0.173,
                    waveform,
                },
            )?;
            vco.set_note(NOTE);
            for _ in 0..sample_rate as usize {
                vco.sample();
            }
            let samples = (0..FRAMES).map(|_| vco.sample()).collect::<Vec<_>>();
            let fundamental_hz = prepared_fundamental_hz(NOTE, sample_rate);
            let nonharmonic_foldback_proxy_db = ratio_db(
                measure_nonharmonic_foldback_proxy(
                    &samples,
                    sample_rate,
                    fundamental_hz,
                    f64::from(sample_rate) * 0.5,
                )
                .ratio,
            );
            rows.push(ModelDVcoAliasEvidence {
                waveform,
                note: NOTE,
                sample_rate,
                asymmetry,
                pulse_width,
                nonharmonic_foldback_proxy_db,
                acceptance_bound_db,
            });
        }
    }
    Ok(rows)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelDCutoffProbe {
    pub target_hz: f32,
    pub measured_hz: f32,
    pub error_fraction: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelDFilterEvidence {
    pub cutoffs: Vec<ModelDCutoffProbe>,
    pub slope_db_per_octave: f32,
    pub resonance_middle_over_low: f32,
    pub resonance_high_over_middle: f32,
}

impl ModelDFilterEvidence {
    pub fn passes(&self) -> bool {
        self.cutoffs.len() == 4
            && self.cutoffs.iter().all(|probe| {
                probe.target_hz.is_finite()
                    && probe.target_hz > 0.0
                    && probe.measured_hz.is_finite()
                    && probe.error_fraction.is_finite()
                    && probe.error_fraction <= MODEL_D_FILTER_CUTOFF_ERROR_MAX
            })
            && self.cutoffs.windows(2).all(|pair| {
                pair[0].measured_hz.is_finite()
                    && pair[1].measured_hz.is_finite()
                    && pair[0].measured_hz < pair[1].measured_hz
            })
            && self.slope_db_per_octave.is_finite()
            && (MODEL_D_FILTER_SLOPE_MIN_DB_PER_OCTAVE..=MODEL_D_FILTER_SLOPE_MAX_DB_PER_OCTAVE)
                .contains(&self.slope_db_per_octave)
            && self.resonance_middle_over_low.is_finite()
            && self.resonance_middle_over_low > MODEL_D_FILTER_RESONANCE_RATIO_MIN
            && self.resonance_high_over_middle.is_finite()
            && self.resonance_high_over_middle > MODEL_D_FILTER_RESONANCE_RATIO_MIN
    }
}

/// Reuses the ladder's declared cutoff, slope, and resonance probes for the
/// offline artifact report. This is engineering evidence, not a hardware match.
pub fn measure_filter_evidence() -> Result<ModelDFilterEvidence, ModelDError> {
    let targets = [125.0_f32, 500.0, 2_000.0, 8_000.0];
    let cutoffs = targets
        .into_iter()
        .map(|target_hz| {
            let measured_hz = measured_ladder_cutoff_hz(target_hz)?;
            Ok(ModelDCutoffProbe {
                target_hz,
                measured_hz,
                error_fraction: (measured_hz - target_hz).abs() / target_hz,
            })
        })
        .collect::<Result<Vec<_>, ModelDError>>()?;
    let lower = ladder_response(250.0, 2_000.0, 0.0)?;
    let upper = ladder_response(250.0, 4_000.0, 0.0)?;
    let resonance_peak = |resonance| -> Result<f32, ModelDError> {
        let passband = ladder_response(1_000.0, 100.0, resonance)?;
        let peak = [1_000.0_f32, 1_500.0, 2_000.0, 2_300.0, 2_500.0, 3_000.0]
            .into_iter()
            .map(|frequency| ladder_response(1_000.0, frequency, resonance))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .fold(0.0_f32, f32::max);
        Ok(peak / passband)
    };
    let low = resonance_peak(0.0)?;
    let middle = resonance_peak(0.45)?;
    let high = resonance_peak(0.8)?;
    Ok(ModelDFilterEvidence {
        cutoffs,
        slope_db_per_octave: 20.0 * (lower / upper).log10(),
        resonance_middle_over_low: middle / low,
        resonance_high_over_middle: high / middle,
    })
}

fn measured_ladder_cutoff_hz(target_hz: f32) -> Result<f32, ModelDError> {
    let threshold = core::f32::consts::FRAC_1_SQRT_2;
    let mut lower = 0.72 * target_hz;
    let mut upper = 1.28 * target_hz;
    for _ in 0..12 {
        let middle = 0.5 * (lower + upper);
        if ladder_response(target_hz, middle, 0.0)? > threshold {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Ok(0.5 * (lower + upper))
}

fn ladder_response(cutoff_hz: f32, frequency_hz: f32, resonance: f32) -> Result<f32, ModelDError> {
    const MIN_CUTOFF_HZ: f32 = 20.0;
    const MAX_CUTOFF_HZ: f32 = 8_000.0;
    let mut ladder = ModelDLadder::new(
        CANONICAL_SAMPLE_RATE as f32,
        LadderConfig {
            resonance,
            drive: 1.0,
            mode: LadderMode::Linear,
        },
    )?;
    let cutoff = (cutoff_hz / MIN_CUTOFF_HZ).ln() / (MAX_CUTOFF_HZ / MIN_CUTOFF_HZ).ln();
    let phase_step = core::f32::consts::TAU * frequency_hz / CANONICAL_SAMPLE_RATE as f32;
    let mut phase = 0.0_f32;
    for _ in 0..8_192 {
        ladder.sample(0.05 * phase.sin(), cutoff);
        phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
    }
    let mut input_sine = 0.0_f64;
    let mut input_cosine = 0.0_f64;
    let mut output_sine = 0.0_f64;
    let mut output_cosine = 0.0_f64;
    for _ in 0..16_384 {
        let input = 0.05 * phase.sin();
        let output = ladder.sample(input, cutoff);
        let sine = f64::from(phase.sin());
        let cosine = f64::from(phase.cos());
        input_sine += f64::from(input) * sine;
        input_cosine += f64::from(input) * cosine;
        output_sine += f64::from(output) * sine;
        output_cosine += f64::from(output) * cosine;
        phase = (phase + phase_step).rem_euclid(core::f32::consts::TAU);
    }
    Ok(output_sine.hypot(output_cosine) as f32 / input_sine.hypot(input_cosine) as f32)
}

pub fn render_audition(kind: AuditionKind, sample_rate: u32) -> Result<ModelDRender, ModelDError> {
    validate_render_sample_rate(sample_rate)?;
    let mono = render_score(kind, sample_rate)?;
    let ablation_residual_rms = if kind.is_ablation() {
        let full = render_score(AuditionKind::FullBassPhrase, sample_rate)?;
        difference_rms(&mono, &full)
    } else {
        0.0
    };
    let mut samples = Vec::with_capacity(mono.len() * 2);
    for sample in &mono {
        samples.extend_from_slice(&[*sample, *sample]);
    }
    let metrics = measure_render(kind, sample_rate, &mono, ablation_residual_rms);
    Ok(ModelDRender { samples, metrics })
}

/// Returns the conservative value of the nonharmonic out-of-mask foldback
/// proxy, using the native reference floor whenever the target estimate is
/// floor-limited. This is not a bound on alias energy inside harmonic masks.
pub fn measure_alias_residual(note: u8) -> Result<f64, ModelDError> {
    let evidence = measure_alias_evidence(note)?;
    Ok(evidence
        .nonharmonic_foldback_proxy_db
        .max(evidence.reference_nonharmonic_floor_db))
}

/// Measures nonharmonic out-of-mask foldback evidence and its explicit
/// harmonic-mask blind spot; it does not bound energy inside masked bins.
pub fn measure_alias_evidence(note: u8) -> Result<ModelDAliasEvidence, ModelDError> {
    measure_alias_evidence_for(note, ModelDAliasProbeConfiguration::ControlledDiagnostic)
}

pub fn measure_full_alias_evidence(note: u8) -> Result<ModelDAliasEvidence, ModelDError> {
    measure_alias_evidence_for(note, ModelDAliasProbeConfiguration::FullAuthoredBassStatic)
}

fn measure_alias_evidence_for(
    note: u8,
    configuration: ModelDAliasProbeConfiguration,
) -> Result<ModelDAliasEvidence, ModelDError> {
    if !(24..=108).contains(&note) {
        return Err(ModelDError::InvalidConfig);
    }
    let low_end = ALIAS_WARMUP_FRAMES_48K + ALIAS_MEASUREMENT_FRAMES;
    let analysis_radius = MODEL_D_ALIAS_ANALYSIS_FILTER_TAPS / 2;
    let high_end = low_end * ANALYSIS_RATE_FACTOR + analysis_radius + ANALYSIS_RATE_FACTOR;
    let low = render_alias_probe(note, CANONICAL_SAMPLE_RATE, low_end, configuration)?;
    let linear_low =
        render_alias_probe_mode(note, CANONICAL_SAMPLE_RATE, low_end, false, configuration)?;
    let high = render_alias_probe(
        note,
        CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
        high_end,
        configuration,
    )?;
    let target = &low[ALIAS_WARMUP_FRAMES_48K..low_end];
    let linear_target = &linear_low[ALIAS_WARMUP_FRAMES_48K..low_end];
    let native_reference_start = ALIAS_WARMUP_FRAMES_48K * ANALYSIS_RATE_FACTOR;
    let native_reference_end =
        native_reference_start + ALIAS_MEASUREMENT_FRAMES * ANALYSIS_RATE_FACTOR;
    let native_reference = &high[native_reference_start..native_reference_end];
    let reference =
        downsample_aligned_reference(&high, ALIAS_WARMUP_FRAMES_48K, ALIAS_MEASUREMENT_FRAMES);
    let target_fundamentals =
        alias_probe_fundamentals_hz(note, CANONICAL_SAMPLE_RATE, configuration);
    let reference_fundamentals = alias_probe_fundamentals_hz(
        note,
        CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
        configuration,
    );
    let target_fundamental_hz = target_fundamentals[0];
    let reference_fundamental_hz = reference_fundamentals[0];
    let full_configuration = matches!(
        configuration,
        ModelDAliasProbeConfiguration::FullAuthoredBassStatic
    );
    let (target_proxy, reference_proxy, proxy_method) = if full_configuration {
        let ultra_factor = ANALYSIS_RATE_FACTOR * ANALYSIS_RATE_FACTOR;
        let ultra_end = low_end * ultra_factor + analysis_radius + ultra_factor;
        let ultra = render_alias_probe(
            note,
            CANONICAL_SAMPLE_RATE * ultra_factor as u32,
            ultra_end,
            configuration,
        )?;
        let ultra_reference = downsample_aligned_reference_factor(
            &ultra,
            ALIAS_WARMUP_FRAMES_48K,
            ALIAS_MEASUREMENT_FRAMES,
            ultra_factor,
        );
        let coverage = harmonic_mask_coverage(
            ALIAS_MEASUREMENT_FRAMES,
            CANONICAL_SAMPLE_RATE,
            &target_fundamentals,
            24_000.0,
        );
        (
            NonharmonicFoldbackProxy {
                ratio: spectral_magnitude_residual_ratio(
                    target,
                    &reference,
                    CANONICAL_SAMPLE_RATE,
                    24_000.0,
                ),
                coverage,
                resolution_hz: f64::from(CANONICAL_SAMPLE_RATE) / ALIAS_MEASUREMENT_FRAMES as f64,
            },
            NonharmonicFoldbackProxy {
                ratio: spectral_magnitude_residual_ratio(
                    &reference,
                    &ultra_reference,
                    CANONICAL_SAMPLE_RATE,
                    24_000.0,
                ),
                coverage,
                resolution_hz: f64::from(CANONICAL_SAMPLE_RATE) / ALIAS_MEASUREMENT_FRAMES as f64,
            },
            "full_path_48_vs_192k_spectral_magnitude_alias_error_with_192_vs_768k_floor",
        )
    } else {
        (
            measure_nonharmonic_foldback_proxy_for_fundamentals(
                target,
                CANONICAL_SAMPLE_RATE,
                &target_fundamentals,
                24_000.0,
            ),
            measure_nonharmonic_foldback_proxy_for_fundamentals(
                native_reference,
                CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
                &reference_fundamentals,
                24_000.0,
            ),
            "controlled_native_nonharmonic_out_of_mask_proxy",
        )
    };
    let (nonharmonic_proxy_floor_limited, excess_nonharmonic_foldback_proxy_db) =
        classify_nonharmonic_foldback_proxy(target_proxy.ratio, reference_proxy.ratio);
    let (source_levels, mixer_drive, ladder_drive) = alias_probe_controls(configuration);
    Ok(ModelDAliasEvidence {
        configuration,
        proxy_method,
        source_levels,
        mixer_drive,
        ladder_drive,
        static_oscillator_imperfections_enabled: matches!(
            configuration,
            ModelDAliasProbeConfiguration::FullAuthoredBassStatic
        ),
        feedback_enabled: matches!(
            configuration,
            ModelDAliasProbeConfiguration::FullAuthoredBassStatic
        ),
        drift_frozen_for_stationary_analysis: true,
        nonharmonic_foldback_proxy_db: ratio_db(target_proxy.ratio),
        reference_nonharmonic_floor_db: ratio_db(reference_proxy.ratio),
        excess_nonharmonic_foldback_proxy_db,
        nonharmonic_proxy_floor_limited,
        transfer_conflated_residual_db: fitted_residual_db(target, &reference),
        target_fundamental_hz,
        reference_fundamental_hz,
        resolution_hz: target_proxy.resolution_hz,
        harmonic_mask_half_width_bins: MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS,
        harmonic_mask_coverage: target_proxy.coverage,
        nonlinear_overtone_magnitude_difference_db: overtone_magnitude_difference_db(
            target,
            linear_target,
            CANONICAL_SAMPLE_RATE,
            target_fundamental_hz,
            24_000.0,
        ),
    })
}

fn classify_nonharmonic_foldback_proxy(
    target_ratio: f64,
    reference_ratio: f64,
) -> (bool, Option<f64>) {
    let floor_limited =
        target_ratio < reference_ratio * MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_POWER_RATIO;
    let excess_db = (!floor_limited).then(|| ratio_db(target_ratio - reference_ratio));
    (floor_limited, excess_db)
}

fn validate_render_sample_rate(sample_rate: u32) -> Result<(), ModelDError> {
    if !(MIN_RENDER_SAMPLE_RATE..=MAX_RENDER_SAMPLE_RATE).contains(&sample_rate) {
        return Err(ModelDError::InvalidSampleRate);
    }
    Ok(())
}

fn render_score(kind: AuditionKind, sample_rate: u32) -> Result<Vec<f32>, ModelDError> {
    let mut patch = kind.patch();
    if kind.uses_bass_score() {
        patch.source_levels[2] = BASS_THIRD_OSCILLATOR_LEVEL;
    }
    let mut voice = ModelDVoice::new(sample_rate as f32, patch, kind.diagnostics())?;
    let frame_count = scale_frame(kind.duration_frames_48k(), sample_rate);
    let mut events = kind.score_events().iter().peekable();
    let mut samples = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        while events
            .peek()
            .is_some_and(|event| scale_frame(event.frame_48k, sample_rate) == frame)
        {
            apply_event(
                &mut voice,
                events.next().expect("peeked score event").action,
            );
        }
        samples.push(voice.sample() * kind.presentation_gain());
    }
    Ok(samples)
}

fn render_alias_probe(
    note: u8,
    sample_rate: u32,
    frame_count: usize,
    configuration: ModelDAliasProbeConfiguration,
) -> Result<Vec<f32>, ModelDError> {
    render_alias_probe_mode(note, sample_rate, frame_count, true, configuration)
}

fn render_alias_probe_mode(
    note: u8,
    sample_rate: u32,
    frame_count: usize,
    nonlinear: bool,
    configuration: ModelDAliasProbeConfiguration,
) -> Result<Vec<f32>, ModelDError> {
    let mut patch = ModelDPatch::bass();
    let (source_levels, mixer_drive, ladder_drive) = alias_probe_controls(configuration);
    patch.source_levels = source_levels;
    patch.mixer_drive = mixer_drive;
    patch.ladder_drive = ladder_drive;
    let full_configuration = matches!(
        configuration,
        ModelDAliasProbeConfiguration::FullAuthoredBassStatic
    );
    let diagnostics = ModelDDiagnostics {
        linear_mixer: !nonlinear,
        linear_ladder: !nonlinear,
        idealize_oscillators: !full_configuration,
        disable_drift: true,
        disable_feedback: !full_configuration,
    };
    let mut voice = ModelDVoice::new(sample_rate as f32, patch, diagnostics)?;
    voice.note_on(note, if full_configuration { 0.88 } else { 0.82 });
    Ok((0..frame_count).map(|_| voice.sample()).collect())
}

fn alias_probe_controls(configuration: ModelDAliasProbeConfiguration) -> ([f32; 3], f32, f32) {
    match configuration {
        ModelDAliasProbeConfiguration::ControlledDiagnostic => ([0.66, 0.54, 0.255], 1.5, 1.5),
        ModelDAliasProbeConfiguration::FullAuthoredBassStatic => {
            ([0.88, 0.72, BASS_THIRD_OSCILLATOR_LEVEL], 2.4, 2.2)
        }
    }
}

fn alias_probe_fundamentals_hz(
    note: u8,
    sample_rate: u32,
    configuration: ModelDAliasProbeConfiguration,
) -> [f64; 3] {
    let offsets_and_cents = match configuration {
        ModelDAliasProbeConfiguration::ControlledDiagnostic => {
            [(-12_i8, 0.0_f64), (0, 0.0), (12, 0.0)]
        }
        ModelDAliasProbeConfiguration::FullAuthoredBassStatic => {
            [(-12_i8, -1.1_f64), (0, 0.8), (12, 1.7)]
        }
    };
    offsets_and_cents.map(|(semitones, cents)| {
        let oscillator_note = (i16::from(note) + i16::from(semitones)).clamp(0, 127) as u8;
        let frequency_hz = midi_frequency(oscillator_note) * 2.0_f64.powf(cents / 1_200.0);
        let increment = frequency_hz as f32 / sample_rate as f32;
        f64::from(increment) * f64::from(sample_rate)
    })
}

fn apply_event(voice: &mut ModelDVoice, action: ScoreAction) {
    match action {
        ScoreAction::NoteOn {
            note,
            velocity_milli,
        } => voice.note_on(note, f32::from(velocity_milli) / 1_000.0),
        ScoreAction::NoteOff => voice.note_off(),
    }
}

fn scale_frame(frame_48k: usize, sample_rate: u32) -> usize {
    ((frame_48k as u64 * u64::from(sample_rate)) / u64::from(CANONICAL_SAMPLE_RATE)) as usize
}

fn measure_render(
    kind: AuditionKind,
    sample_rate: u32,
    samples: &[f32],
    ablation_residual_rms: f64,
) -> ModelDMetrics {
    let count = samples.len() as f64;
    let finite = samples.iter().all(|sample| sample.is_finite());
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0_f64, f64::max);
    let rms = (samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>()
        / count)
        .sqrt();
    let dc = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / count;
    let maximum_jump = samples
        .windows(2)
        .map(|pair| f64::from((pair[1] - pair[0]).abs()))
        .fold(0.0_f64, f64::max);
    let headroom_db = -20.0 * peak.max(1.0e-24).log10();
    let zero_tail_frames = samples
        .iter()
        .rev()
        .take_while(|sample| **sample == 0.0)
        .count();
    let expected_pitch_hz = midi_frequency(kind.expected_pitch_note());
    let first_note_off_48k = kind
        .score_events()
        .iter()
        .find_map(|event| matches!(event.action, ScoreAction::NoteOff).then_some(event.frame_48k))
        .unwrap_or(24_000);
    let patch = kind.patch();
    let pitch_start_48k =
        ((patch.filter_contour.attack_seconds + patch.filter_contour.decay_seconds + 0.010)
            * CANONICAL_SAMPLE_RATE as f32)
            .round() as usize;
    let pitch_hz = estimate_first_note_pitch(
        samples,
        sample_rate,
        expected_pitch_hz,
        pitch_start_48k,
        first_note_off_48k,
    );
    let pitch_error_cents = 1_200.0 * (pitch_hz / expected_pitch_hz).log2();
    ModelDMetrics {
        peak,
        rms,
        dc,
        maximum_jump,
        headroom_db,
        pitch_hz,
        pitch_error_cents,
        presentation_gain: kind.presentation_gain(),
        zero_tail_frames,
        ablation_residual_rms,
        sample_hash: hash_sample_stream(samples),
        score_hash: hash_score(kind.score_events(), kind.duration_frames_48k()),
        finite,
    }
}

fn estimate_first_note_pitch(
    samples: &[f32],
    sample_rate: u32,
    expected_hz: f64,
    requested_start_48k: usize,
    first_note_off_48k: usize,
) -> f64 {
    let end_margin_48k = 2_400.min(first_note_off_48k / 6);
    let maximum_start_48k = first_note_off_48k.saturating_sub(2 * end_margin_48k);
    let start = scale_frame(requested_start_48k.min(maximum_start_48k), sample_rate);
    let end = scale_frame(first_note_off_48k - end_margin_48k, sample_rate).min(samples.len());
    let window = &samples[start..end];
    let mean = window.iter().map(|sample| f64::from(*sample)).sum::<f64>() / window.len() as f64;
    const SEARCH_STEPS: usize = 160;
    const SEARCH_CENTS: f64 = 20.0;
    (0..=SEARCH_STEPS)
        .map(|step| {
            let cents = -SEARCH_CENTS + 2.0 * SEARCH_CENTS * step as f64 / SEARCH_STEPS as f64;
            expected_hz * 2.0_f64.powf(cents / 1_200.0)
        })
        .max_by(|left, right| {
            windowed_tone_energy(window, mean, *left, sample_rate).total_cmp(&windowed_tone_energy(
                window,
                mean,
                *right,
                sample_rate,
            ))
        })
        .unwrap_or(expected_hz)
}

fn windowed_tone_energy(samples: &[f32], mean: f64, frequency_hz: f64, sample_rate: u32) -> f64 {
    let angle = std::f64::consts::TAU * frequency_hz / f64::from(sample_rate);
    let (rotation_sine, rotation_cosine) = angle.sin_cos();
    let mut sine = 0.0_f64;
    let mut cosine = 1.0_f64;
    let mut sine_projection = 0.0_f64;
    let mut cosine_projection = 0.0_f64;
    for (index, sample) in samples.iter().enumerate() {
        let window_phase = std::f64::consts::TAU * index as f64 / (samples.len() - 1) as f64;
        let window = 0.358_75 - 0.488_29 * window_phase.cos()
            + 0.141_28 * (2.0 * window_phase).cos()
            - 0.011_68 * (3.0 * window_phase).cos();
        let value = (f64::from(*sample) - mean) * window;
        sine_projection += value * sine;
        cosine_projection += value * cosine;
        let next_sine = sine * rotation_cosine + cosine * rotation_sine;
        cosine = cosine * rotation_cosine - sine * rotation_sine;
        sine = next_sine;
    }
    sine_projection * sine_projection + cosine_projection * cosine_projection
}

fn difference_rms(left: &[f32], right: &[f32]) -> f64 {
    (left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let difference = f64::from(*left) - f64::from(*right);
            difference * difference
        })
        .sum::<f64>()
        / left.len().max(1) as f64)
        .sqrt()
}

pub fn hash_sample_stream(samples: &[f32]) -> u64 {
    samples.iter().fold(0xcbf2_9ce4_8422_2325, |hash, sample| {
        (hash ^ u64::from(sample.to_bits())).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn hash_score(events: &[ScoreEvent], duration_frames: usize) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for event in events {
        hash = (hash ^ event.frame_48k as u64).wrapping_mul(0x0000_0100_0000_01b3);
        let action = match event.action {
            ScoreAction::NoteOn {
                note,
                velocity_milli,
            } => (1_u64 << 63) | (u64::from(note) << 16) | u64::from(velocity_milli),
            ScoreAction::NoteOff => 0,
        };
        hash = (hash ^ action).wrapping_mul(0x0000_0100_0000_01b3);
    }
    (hash ^ duration_frames as u64).wrapping_mul(0x0000_0100_0000_01b3)
}

fn midi_frequency(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((f64::from(note) - 69.0) / 12.0)
}

fn prepared_fundamental_hz(note: u8, sample_rate: u32) -> f64 {
    let semitones_from_a4 = f32::from(note) - 69.0;
    let frequency_hz = 440.0_f32 * 2.0_f32.powf(semitones_from_a4 / 12.0);
    let increment = frequency_hz / sample_rate as f32;
    f64::from(increment) * f64::from(sample_rate)
}

#[derive(Clone, Copy, Debug)]
struct NonharmonicFoldbackProxy {
    ratio: f64,
    coverage: f64,
    resolution_hz: f64,
}

fn measure_nonharmonic_foldback_proxy(
    samples: &[f32],
    sample_rate: u32,
    fundamental_hz: f64,
    maximum_hz: f64,
) -> NonharmonicFoldbackProxy {
    measure_nonharmonic_foldback_proxy_for_fundamentals(
        samples,
        sample_rate,
        &[fundamental_hz],
        maximum_hz,
    )
}

fn measure_nonharmonic_foldback_proxy_for_fundamentals(
    samples: &[f32],
    sample_rate: u32,
    fundamental_frequencies_hz: &[f64],
    maximum_hz: f64,
) -> NonharmonicFoldbackProxy {
    assert!(samples.len().is_power_of_two());
    let sample_count = samples.len();
    let resolution_hz = f64::from(sample_rate) / sample_count as f64;
    let spectrum = windowed_spectrum(samples);
    let maximum_bin = (maximum_hz / resolution_hz).floor() as usize;
    let harmonic_mask = harmonic_mask(maximum_bin, resolution_hz, fundamental_frequencies_hz);

    let mut total_energy = 0.0_f64;
    let mut residual_energy = 0.0_f64;
    for bin in 1..=maximum_bin {
        let energy = spectrum[bin].0 * spectrum[bin].0 + spectrum[bin].1 * spectrum[bin].1;
        total_energy += energy;
        if !harmonic_mask[bin] {
            residual_energy += energy;
        }
    }
    let masked_bins = harmonic_mask[1..].iter().filter(|masked| **masked).count();
    NonharmonicFoldbackProxy {
        ratio: (residual_energy / total_energy.max(1.0e-24)).max(1.0e-24),
        coverage: masked_bins as f64 / maximum_bin.max(1) as f64,
        resolution_hz,
    }
}

fn harmonic_mask_coverage(
    sample_count: usize,
    sample_rate: u32,
    fundamental_frequencies_hz: &[f64],
    maximum_hz: f64,
) -> f64 {
    let resolution_hz = f64::from(sample_rate) / sample_count as f64;
    let maximum_bin = (maximum_hz / resolution_hz).floor() as usize;
    let mask = harmonic_mask(maximum_bin, resolution_hz, fundamental_frequencies_hz);
    mask[1..].iter().filter(|masked| **masked).count() as f64 / maximum_bin.max(1) as f64
}

fn spectral_magnitude_residual_ratio(
    target: &[f32],
    reference: &[f32],
    sample_rate: u32,
    maximum_hz: f64,
) -> f64 {
    assert_eq!(target.len(), reference.len());
    let resolution_hz = f64::from(sample_rate) / target.len() as f64;
    let maximum_bin = (maximum_hz / resolution_hz).floor() as usize;
    let target_spectrum = windowed_spectrum(target);
    let reference_spectrum = windowed_spectrum(reference);
    let magnitudes = |spectrum: &[(f64, f64)]| {
        spectrum[1..=maximum_bin]
            .iter()
            .map(|&(real, imaginary)| real.hypot(imaginary))
            .collect::<Vec<_>>()
    };
    let target_magnitudes = magnitudes(&target_spectrum);
    let reference_magnitudes = magnitudes(&reference_spectrum);
    let dot = target_magnitudes
        .iter()
        .zip(&reference_magnitudes)
        .map(|(target, reference)| target * reference)
        .sum::<f64>();
    let reference_energy = reference_magnitudes
        .iter()
        .map(|magnitude| magnitude * magnitude)
        .sum::<f64>();
    let gain = dot / reference_energy.max(1.0e-24);
    let target_energy = target_magnitudes
        .iter()
        .map(|magnitude| magnitude * magnitude)
        .sum::<f64>();
    let residual_energy = target_magnitudes
        .iter()
        .zip(&reference_magnitudes)
        .map(|(target, reference)| {
            let residual = target - gain * reference;
            residual * residual
        })
        .sum::<f64>();
    (residual_energy / target_energy.max(1.0e-24)).max(1.0e-24)
}

fn windowed_spectrum(samples: &[f32]) -> Vec<(f64, f64)> {
    assert!(samples.len().is_power_of_two());
    let sample_count = samples.len();
    let mean = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / sample_count as f64;
    let mut spectrum: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let phase = std::f64::consts::TAU * index as f64 / (sample_count - 1) as f64;
            let window = 0.358_75 - 0.488_29 * phase.cos() + 0.141_28 * (2.0 * phase).cos()
                - 0.011_68 * (3.0 * phase).cos();
            ((f64::from(*sample) - mean) * window, 0.0_f64)
        })
        .collect();
    fft_in_place(&mut spectrum);
    spectrum
}

fn harmonic_mask(
    maximum_bin: usize,
    resolution_hz: f64,
    fundamental_frequencies_hz: &[f64],
) -> Vec<bool> {
    let mut harmonic_mask = vec![false; maximum_bin + 1];
    for &fundamental_hz in fundamental_frequencies_hz {
        let harmonic_count =
            ((maximum_bin as f64 * resolution_hz) / fundamental_hz).floor() as usize;
        for harmonic in 1..=harmonic_count {
            let center = (fundamental_hz * harmonic as f64 / resolution_hz).round() as usize;
            let first = center
                .saturating_sub(MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS)
                .max(1);
            let last = (center + MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS).min(maximum_bin);
            harmonic_mask[first..=last].fill(true);
        }
    }
    harmonic_mask
}

fn overtone_magnitude_difference_db(
    target: &[f32],
    reference: &[f32],
    sample_rate: u32,
    fundamental_hz: f64,
    maximum_hz: f64,
) -> f64 {
    assert_eq!(target.len(), reference.len());
    let resolution_hz = f64::from(sample_rate) / target.len() as f64;
    let target_spectrum = windowed_spectrum(target);
    let reference_spectrum = windowed_spectrum(reference);
    let maximum_bin = (maximum_hz / resolution_hz).floor() as usize;
    let harmonic_count = (maximum_hz / fundamental_hz).floor() as usize;
    let band_magnitudes = |spectrum: &[(f64, f64)]| {
        (1..=harmonic_count)
            .map(|harmonic| {
                let center = (fundamental_hz * harmonic as f64 / resolution_hz).round() as usize;
                let first = center
                    .saturating_sub(MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS)
                    .max(1);
                let last = (center + MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS).min(maximum_bin);
                spectrum[first..=last]
                    .iter()
                    .map(|&(real, imaginary)| real * real + imaginary * imaginary)
                    .sum::<f64>()
                    .sqrt()
            })
            .collect::<Vec<_>>()
    };
    let target_magnitudes = band_magnitudes(&target_spectrum);
    let reference_magnitudes = band_magnitudes(&reference_spectrum);
    let gain = target_magnitudes[0] / reference_magnitudes[0].max(1.0e-24);
    let difference_energy = target_magnitudes
        .iter()
        .zip(&reference_magnitudes)
        .skip(1)
        .map(|(&target, &reference)| {
            let difference = target - gain * reference;
            difference * difference
        })
        .sum::<f64>();
    let target_harmonic_energy = target_magnitudes
        .iter()
        .map(|magnitude| magnitude * magnitude)
        .sum::<f64>();
    ratio_db((difference_energy / target_harmonic_energy.max(1.0e-24)).max(1.0e-24))
}

fn fft_in_place(values: &mut [(f64, f64)]) {
    let sample_count = values.len();
    let mut reversed = 0;
    for index in 1..sample_count {
        let mut bit = sample_count >> 1;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit >>= 1;
        }
        reversed ^= bit;
        if index < reversed {
            values.swap(index, reversed);
        }
    }

    let mut block_size = 2;
    while block_size <= sample_count {
        let angle = -std::f64::consts::TAU / block_size as f64;
        let (step_imaginary, step_real) = angle.sin_cos();
        for block_start in (0..sample_count).step_by(block_size) {
            let mut twiddle_real = 1.0_f64;
            let mut twiddle_imaginary = 0.0_f64;
            for offset in 0..block_size / 2 {
                let even = values[block_start + offset];
                let odd = values[block_start + offset + block_size / 2];
                let odd_real = odd.0 * twiddle_real - odd.1 * twiddle_imaginary;
                let odd_imaginary = odd.0 * twiddle_imaginary + odd.1 * twiddle_real;
                values[block_start + offset] = (even.0 + odd_real, even.1 + odd_imaginary);
                values[block_start + offset + block_size / 2] =
                    (even.0 - odd_real, even.1 - odd_imaginary);
                let next_real = twiddle_real * step_real - twiddle_imaginary * step_imaginary;
                twiddle_imaginary = twiddle_real * step_imaginary + twiddle_imaginary * step_real;
                twiddle_real = next_real;
            }
        }
        block_size *= 2;
    }
}

fn ratio_db(ratio: f64) -> f64 {
    10.0 * ratio.max(1.0e-24).log10()
}

fn downsample_aligned_reference(
    high_rate: &[f32],
    first_low_rate_frame: usize,
    output_frames: usize,
) -> Vec<f32> {
    downsample_aligned_reference_factor(
        high_rate,
        first_low_rate_frame,
        output_frames,
        ANALYSIS_RATE_FACTOR,
    )
}

fn downsample_aligned_reference_factor(
    high_rate: &[f32],
    first_low_rate_frame: usize,
    output_frames: usize,
    rate_factor: usize,
) -> Vec<f32> {
    let analysis_radius = (MODEL_D_ALIAS_ANALYSIS_FILTER_TAPS / 2) as isize;
    let ladder_alignment_high_rate =
        (rate_factor as f64 - 1.0) - (rate_factor as f64 - 1.0) * MODEL_D_LADDER_GROUP_DELAY_HOST;
    (0..output_frames)
        .map(|output_frame| {
            let low_frame = first_low_rate_frame + output_frame;
            let center = low_frame as f64 * rate_factor as f64 + ladder_alignment_high_rate;
            let center_floor = center.floor() as isize;
            let mut output = 0.0_f64;
            let mut weight = 0.0_f64;
            for tap_offset in -analysis_radius..=analysis_radius {
                let index = center_floor + tap_offset;
                if !(0..high_rate.len() as isize).contains(&index) {
                    continue;
                }
                let distance = index as f64 - center;
                let analysis_cutoff = MODEL_D_ALIAS_ANALYSIS_FILTER_CUTOFF_HZ
                    / (f64::from(CANONICAL_SAMPLE_RATE) * rate_factor as f64);
                let sinc_argument = 2.0 * std::f64::consts::PI * analysis_cutoff * distance;
                let sinc = if sinc_argument.abs() < 1.0e-12 {
                    2.0 * analysis_cutoff
                } else {
                    sinc_argument.sin() / (std::f64::consts::PI * distance)
                };
                let normalized_offset = tap_offset as f64 / analysis_radius as f64;
                let window = 0.42
                    + 0.5 * (std::f64::consts::PI * normalized_offset).cos()
                    + 0.08 * (2.0 * std::f64::consts::PI * normalized_offset).cos();
                let coefficient = sinc * window;
                output += f64::from(high_rate[index as usize]) * coefficient;
                weight += coefficient;
            }
            (output / weight.max(1.0e-24)) as f32
        })
        .collect()
}

fn fitted_residual_db(target: &[f32], reference: &[f32]) -> f64 {
    let target_mean =
        target.iter().map(|sample| f64::from(*sample)).sum::<f64>() / target.len() as f64;
    let reference_mean = reference
        .iter()
        .map(|sample| f64::from(*sample))
        .sum::<f64>()
        / reference.len() as f64;
    let mut dot = 0.0_f64;
    let mut reference_energy = 0.0_f64;
    let mut target_energy = 0.0_f64;
    for (&target, &reference) in target.iter().zip(reference) {
        let target = f64::from(target) - target_mean;
        let reference = f64::from(reference) - reference_mean;
        dot += target * reference;
        reference_energy += reference * reference;
        target_energy += target * target;
    }
    let gain = if reference_energy > 0.0 {
        dot / reference_energy
    } else {
        0.0
    };
    let residual_energy = target
        .iter()
        .zip(reference)
        .map(|(&target, &reference)| {
            let residual =
                (f64::from(target) - target_mean) - gain * (f64::from(reference) - reference_mean);
            residual * residual
        })
        .sum::<f64>();
    10.0 * (residual_energy / target_energy.max(1.0e-24))
        .max(1.0e-24)
        .log10()
}

#[cfg(test)]
mod tests {
    use crate::model_d::vco::{ModelDVco, ModelDWaveform, VcoConfig};

    use super::{
        AuditionKind, ModelDAliasProbeConfiguration, measure_alias_evidence,
        measure_alias_residual, measure_filter_evidence, measure_full_alias_evidence,
        measure_vco_pitch_matrix, measure_vco_waveform_alias_matrix, render_audition,
    };

    const SAMPLE_RATE: u32 = 48_000;
    const MAX_PEAK_FOR_ONE_DB_HEADROOM: f64 = 0.891_250_938_133_745_6;

    fn waveform_alias_probe(waveform: ModelDWaveform, sample_rate: u32, asymmetry: f32) -> f64 {
        const FRAMES: usize = 131_072;
        let note = 96;
        let mut vco = ModelDVco::new(
            sample_rate as f32,
            VcoConfig {
                semitone_offset: 0,
                cents_offset: 0.0,
                drift_cents: 0.0,
                drift_hz: 0.25,
                asymmetry,
                level_offset: 0.0,
                reset_phase: 0.173,
                waveform,
            },
        )
        .unwrap();
        vco.set_note(note);
        for _ in 0..sample_rate as usize {
            vco.sample();
        }
        let samples = (0..FRAMES).map(|_| vco.sample()).collect::<Vec<_>>();
        let fundamental_hz = super::prepared_fundamental_hz(note, sample_rate);
        super::ratio_db(
            super::measure_nonharmonic_foldback_proxy(
                &samples,
                sample_rate,
                fundamental_hz,
                f64::from(sample_rate) * 0.5,
            )
            .ratio,
        )
    }

    #[test]
    fn every_distinct_vco_waveform_has_bounded_high_note_nonharmonic_foldback() {
        let mut failures = Vec::new();
        for sample_rate in [44_100, 48_000, 96_000] {
            for (waveform, asymmetry, maximum_db) in [
                (ModelDWaveform::Triangle, 1.0, -40.0),
                (ModelDWaveform::Saw, 1.0, -28.0),
                (ModelDWaveform::Rectangle, 0.0, -28.0),
                (ModelDWaveform::WidePulse, 1.0, -24.0),
                (ModelDWaveform::NarrowPulse, -1.0, -24.0),
            ] {
                let residual_db = waveform_alias_probe(waveform, sample_rate, asymmetry);
                eprintln!(
                    "waveform={waveform:?}, sample_rate={sample_rate}, asymmetry={asymmetry}, residual_db={residual_db}, maximum_db={maximum_db}"
                );
                if residual_db > maximum_db {
                    failures.push((waveform, sample_rate, asymmetry, residual_db, maximum_db));
                }
            }
        }
        assert!(failures.is_empty(), "failures={failures:?}");
    }

    #[test]
    fn oscillator_pitch_and_drift_evidence_covers_the_exact_required_matrix() {
        let rows = measure_vco_pitch_matrix().unwrap();
        assert_eq!(rows.len(), 9, "{rows:?}");
        assert_eq!(
            rows.iter()
                .map(|row| (row.note, row.sample_rate))
                .collect::<Vec<_>>(),
            [
                (36, 44_100),
                (60, 44_100),
                (84, 44_100),
                (36, 48_000),
                (60, 48_000),
                (84, 48_000),
                (36, 96_000),
                (60, 96_000),
                (84, 96_000),
            ]
        );
        for row in rows {
            assert_eq!(row.configured_static_cents, -2.0, "{row:?}");
            assert_eq!(row.configured_drift_cents, 1.5, "{row:?}");
            assert!(row.mean_pitch_error_cents.abs() <= 5.0, "{row:?}");
            assert!(row.minimum_drift_cents >= -1.5, "{row:?}");
            assert!(row.maximum_drift_cents <= 1.5, "{row:?}");
            assert!(
                row.maximum_drift_cents - row.minimum_drift_cents >= 2.9,
                "{row:?}"
            );
        }
    }

    #[test]
    fn public_waveform_alias_matrix_reports_every_waveform_rate_and_bound() {
        let rows = measure_vco_waveform_alias_matrix().unwrap();
        assert_eq!(rows.len(), 15, "{rows:?}");
        for sample_rate in [44_100, 48_000, 96_000] {
            let at_rate = rows
                .iter()
                .filter(|row| row.sample_rate == sample_rate)
                .collect::<Vec<_>>();
            assert_eq!(at_rate.len(), 5, "sample_rate={sample_rate}: {rows:?}");
            assert_eq!(
                at_rate.iter().map(|row| row.waveform).collect::<Vec<_>>(),
                [
                    ModelDWaveform::Triangle,
                    ModelDWaveform::Saw,
                    ModelDWaveform::Rectangle,
                    ModelDWaveform::WidePulse,
                    ModelDWaveform::NarrowPulse,
                ]
            );
        }
        assert!(rows.iter().all(|row| {
            row.note == 96
                && row.nonharmonic_foldback_proxy_db.is_finite()
                && row.nonharmonic_foldback_proxy_db <= row.acceptance_bound_db
        }));
    }

    #[test]
    fn full_authored_bass_alias_evidence_keeps_static_character_drive_and_feedback() {
        let mut failing_notes = Vec::new();
        for (note, maximum_db) in [(36, -45.0), (60, -45.0), (84, -35.0)] {
            let evidence = measure_full_alias_evidence(note).unwrap();
            assert_eq!(
                evidence.configuration,
                ModelDAliasProbeConfiguration::FullAuthoredBassStatic
            );
            assert_eq!(evidence.source_levels, [0.88, 0.72, 0.14]);
            assert_eq!(evidence.mixer_drive, 2.4);
            assert_eq!(evidence.ladder_drive, 2.2);
            assert!(evidence.static_oscillator_imperfections_enabled);
            assert!(evidence.feedback_enabled);
            assert!(evidence.drift_frozen_for_stationary_analysis);
            let conservative = evidence
                .nonharmonic_foldback_proxy_db
                .max(evidence.reference_nonharmonic_floor_db);
            eprintln!(
                "note={note}, maximum_db={maximum_db}, conservative_db={conservative}, evidence={evidence:?}"
            );
            if conservative > maximum_db {
                failing_notes.push((note, conservative, maximum_db));
            }
            assert!(
                (super::MODEL_D_FULL_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB
                    ..=super::MODEL_D_FULL_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB)
                    .contains(&evidence.nonlinear_overtone_magnitude_difference_db),
                "note={note}, evidence={evidence:?}"
            );
        }
        assert!(
            !failing_notes.is_empty(),
            "full-path evidence must remain explicitly diagnostic until it meets its unchanged bounds"
        );
    }

    #[test]
    fn audition_contract_has_exactly_seven_stably_named_files() {
        assert_eq!(
            AuditionKind::ALL.map(AuditionKind::filename),
            [
                "01_full_bass_phrase.wav",
                "02_full_lead_phrase.wav",
                "03_full_filter_articulation.wav",
                "04_matched_idealized_path.wav",
                "05_matched_linear_mixer.wav",
                "06_matched_linear_ladder.wav",
                "07_matched_no_drift_or_feedback.wav",
            ]
        );
    }

    #[test]
    fn public_filter_evidence_reuses_ladder_gates_and_rejects_nonfinite_values() {
        let evidence = measure_filter_evidence().unwrap();
        assert!(evidence.passes(), "{evidence:?}");
        assert!(
            evidence.cutoffs.iter().all(|probe| {
                probe.target_hz.is_finite()
                    && probe.measured_hz.is_finite()
                    && probe.error_fraction.is_finite()
            }),
            "{evidence:?}"
        );
        assert!(evidence.slope_db_per_octave.is_finite());
        assert!(evidence.resonance_middle_over_low.is_finite());
        assert!(evidence.resonance_high_over_middle.is_finite());

        for invalidate in [
            |evidence: &mut super::ModelDFilterEvidence| {
                evidence.cutoffs[0].measured_hz = f32::NAN;
            },
            |evidence: &mut super::ModelDFilterEvidence| {
                evidence.slope_db_per_octave = f32::NAN;
            },
            |evidence: &mut super::ModelDFilterEvidence| {
                evidence.resonance_middle_over_low = f32::NAN;
            },
            |evidence: &mut super::ModelDFilterEvidence| {
                evidence.resonance_high_over_middle = f32::NAN;
            },
        ] {
            let mut invalid = evidence.clone();
            invalidate(&mut invalid);
            assert!(!invalid.passes(), "{invalid:?}");
        }
    }

    #[test]
    fn matched_files_share_the_bass_score_and_fixed_gain() {
        let matched = [
            AuditionKind::FullBassPhrase,
            AuditionKind::MatchedIdealizedPath,
            AuditionKind::MatchedLinearMixer,
            AuditionKind::MatchedLinearLadder,
            AuditionKind::MatchedNoDriftOrFeedback,
        ];
        let expected_score = matched[0].score_name();
        let expected_gain = matched[0].presentation_gain();
        assert!(expected_gain.is_finite() && expected_gain > 0.0);
        for kind in matched {
            assert_eq!(kind.score_name(), expected_score);
            assert_eq!(kind.presentation_gain(), expected_gain);
        }
    }

    #[test]
    fn authored_scores_are_explicit_monophonic_events_with_long_zero_tails() {
        for kind in [
            AuditionKind::FullBassPhrase,
            AuditionKind::FullLeadPhrase,
            AuditionKind::FullFilterArticulation,
        ] {
            let events = kind.score_events();
            assert!(events.len() >= 6, "{kind:?}");
            assert_eq!(events.first().map(|event| event.frame_48k), Some(0));
            assert!(
                events
                    .windows(2)
                    .all(|pair| pair[0].frame_48k < pair[1].frame_48k),
                "{kind:?}: {events:?}"
            );
            let mut active = false;
            for event in events {
                match event.action {
                    super::ScoreAction::NoteOn { .. } => {
                        assert!(!active, "{kind:?}: overlapping note-on");
                        active = true;
                    }
                    super::ScoreAction::NoteOff => {
                        assert!(active, "{kind:?}: note-off without note-on");
                        active = false;
                    }
                }
            }
            assert!(!active, "{kind:?}: final note remains active");
            assert!(
                kind.duration_frames_48k() - events.last().unwrap().frame_48k
                    >= SAMPLE_RATE as usize
            );
        }
    }

    #[test]
    fn every_audition_is_safe_stereo_deterministic_and_pitch_measured() {
        for kind in AuditionKind::ALL {
            let first = render_audition(kind, SAMPLE_RATE).unwrap();
            let second = render_audition(kind, SAMPLE_RATE).unwrap();
            assert_eq!(first.samples, second.samples, "{kind:?}");
            assert_eq!(first.metrics, second.metrics, "{kind:?}");
            assert_eq!(first.samples.len() % 2, 0, "{kind:?}");
            assert_eq!(
                first.samples.len() / 2,
                kind.duration_frames_48k(),
                "{kind:?}"
            );
            assert!(
                first
                    .samples
                    .chunks_exact(2)
                    .all(|frame| frame[0] == frame[1])
            );
            assert!(first.metrics.finite, "{kind:?}: {:?}", first.metrics);
            assert!(first.metrics.peak > 1.0e-4, "{kind:?}: {:?}", first.metrics);
            assert!(
                first.metrics.peak <= MAX_PEAK_FOR_ONE_DB_HEADROOM,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.headroom_db >= 1.0,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.dc.abs() <= 0.005,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.maximum_jump <= 0.25,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.zero_tail_frames >= SAMPLE_RATE as usize / 2,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.pitch_hz.is_finite() && first.metrics.pitch_hz > 0.0,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert!(
                first.metrics.pitch_error_cents.abs() <= 10.0,
                "{kind:?}: {:?}",
                first.metrics
            );
            assert_ne!(first.metrics.sample_hash, 0, "{kind:?}");
        }
    }

    #[test]
    fn matched_ablation_residuals_are_nonzero_without_score_or_gain_changes() {
        let full = render_audition(AuditionKind::FullBassPhrase, SAMPLE_RATE).unwrap();
        for kind in [
            AuditionKind::MatchedIdealizedPath,
            AuditionKind::MatchedLinearMixer,
            AuditionKind::MatchedLinearLadder,
            AuditionKind::MatchedNoDriftOrFeedback,
        ] {
            let ablation = render_audition(kind, SAMPLE_RATE).unwrap();
            assert_eq!(ablation.samples.len(), full.samples.len(), "{kind:?}");
            assert_eq!(
                ablation.metrics.presentation_gain, full.metrics.presentation_gain,
                "{kind:?}"
            );
            assert_eq!(
                ablation.metrics.score_hash, full.metrics.score_hash,
                "{kind:?}"
            );
            assert!(
                ablation.metrics.ablation_residual_rms > 1.0e-7,
                "{kind:?}: {:?}",
                ablation.metrics
            );
        }
    }

    #[test]
    fn invalid_render_rates_are_rejected() {
        assert!(render_audition(AuditionKind::FullBassPhrase, 0).is_err());
        assert!(render_audition(AuditionKind::FullBassPhrase, u32::MAX).is_err());
    }

    #[test]
    fn steady_nonlinear_high_rate_nonharmonic_proxy_is_classified_and_within_bounds() {
        for (note, maximum_db) in [(36, -45.0), (60, -45.0), (84, -35.0)] {
            let evidence = measure_alias_evidence(note).unwrap();
            let first = measure_alias_residual(note).unwrap();
            let second = measure_alias_residual(note).unwrap();
            assert_eq!(first, second, "note={note}");
            assert_eq!(
                first,
                evidence
                    .nonharmonic_foldback_proxy_db
                    .max(evidence.reference_nonharmonic_floor_db),
                "note={note}"
            );
            assert!(first.is_finite(), "note={note}, residual_db={first}");
            assert!(
                first <= maximum_db,
                "note={note}, residual_db={first}, maximum_db={maximum_db}, evidence={evidence:?}"
            );
            assert!(
                evidence.reference_nonharmonic_floor_db <= maximum_db,
                "note={note}, evidence={evidence:?}"
            );
            if evidence.nonharmonic_proxy_floor_limited {
                assert_eq!(
                    evidence.excess_nonharmonic_foldback_proxy_db, None,
                    "{evidence:?}"
                );
                assert!(
                    evidence.nonharmonic_foldback_proxy_db
                        < evidence.reference_nonharmonic_floor_db
                            + super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB,
                    "{evidence:?}"
                );
            } else {
                assert!(
                    evidence.reference_nonharmonic_floor_db
                        <= evidence.nonharmonic_foldback_proxy_db
                            - super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB,
                    "note={note}, evidence={evidence:?}"
                );
                assert!(
                    evidence
                        .excess_nonharmonic_foldback_proxy_db
                        .is_some_and(|db| db <= maximum_db),
                    "note={note}, evidence={evidence:?}"
                );
            }
            assert!(
                evidence.transfer_conflated_residual_db.is_finite(),
                "note={note}, evidence={evidence:?}"
            );
            assert!(
                evidence.target_fundamental_hz.is_finite()
                    && evidence.reference_fundamental_hz.is_finite(),
                "note={note}, evidence={evidence:?}"
            );
            assert!(
                (evidence.target_fundamental_hz / evidence.reference_fundamental_hz)
                    .log2()
                    .abs()
                    * 1_200.0
                    <= 5.0,
                "note={note}, evidence={evidence:?}"
            );
            assert_eq!(evidence.harmonic_mask_half_width_bins, 4);
            assert!(evidence.resolution_hz <= 0.37, "{evidence:?}");
            assert!(evidence.harmonic_mask_coverage <= 0.15, "{evidence:?}");
            assert!(
                (super::MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MIN_DB
                    ..=super::MODEL_D_NONLINEAR_OVERTONE_DIFFERENCE_MAX_DB)
                    .contains(&evidence.nonlinear_overtone_magnitude_difference_db),
                "note={note}, evidence={evidence:?}"
            );
            assert_eq!(
                evidence.nonharmonic_proxy_floor_limited,
                note == 36,
                "{evidence:?}"
            );
        }
        assert!(measure_alias_residual(127).is_err());
    }

    #[test]
    fn reference_margin_classification_includes_below_exact_and_above_boundary() {
        assert!(
            (super::ratio_db(super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_POWER_RATIO)
                - super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB)
                .abs()
                <= f64::EPSILON
        );
        let reference = 1.0e-8;
        let required = reference * super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_POWER_RATIO;
        assert_eq!(
            super::classify_nonharmonic_foldback_proxy(required * (1.0 - 1.0e-12), reference),
            (true, None)
        );
        let exact = super::classify_nonharmonic_foldback_proxy(required, reference);
        assert!(!exact.0);
        assert!(exact.1.is_some());
        let above =
            super::classify_nonharmonic_foldback_proxy(required * (1.0 + 1.0e-12), reference);
        assert!(!above.0);
        assert!(above.1.is_some());
    }

    #[test]
    fn note_36_proxy_detects_off_grid_foldback_but_excludes_on_grid_energy() {
        const FRAMES: usize = 131_072;
        const FOLDED_LEVEL: f64 = 0.01;
        let fundamental_hz = super::prepared_fundamental_hz(24, SAMPLE_RATE);
        let folded_frequencies = [1_000.0, 8_000.0, 18_000.0]
            .map(|near_hz| ((near_hz / fundamental_hz).floor() + 0.5) * fundamental_hz);
        let on_grid_frequencies = [1_000.0, 8_000.0, 18_000.0]
            .map(|near_hz| (near_hz / fundamental_hz).round() * fundamental_hz);
        let clean: Vec<_> = (0..FRAMES)
            .map(|frame| {
                let phase =
                    std::f64::consts::TAU * fundamental_hz * frame as f64 / f64::from(SAMPLE_RATE);
                (phase.sin() + 0.3 * (2.0 * phase).sin() + 0.2 * (3.0 * phase).sin()) as f32
            })
            .collect();
        let folded: Vec<_> = clean
            .iter()
            .enumerate()
            .map(|(frame, clean)| {
                folded_frequencies.iter().fold(*clean, |sample, frequency| {
                    sample
                        + (FOLDED_LEVEL
                            * (std::f64::consts::TAU * frequency * frame as f64
                                / f64::from(SAMPLE_RATE))
                            .sin()) as f32
                })
            })
            .collect();
        let on_grid: Vec<_> = clean
            .iter()
            .enumerate()
            .map(|(frame, clean)| {
                on_grid_frequencies
                    .iter()
                    .fold(*clean, |sample, frequency| {
                        sample
                            + (FOLDED_LEVEL
                                * (std::f64::consts::TAU * frequency * frame as f64
                                    / f64::from(SAMPLE_RATE))
                                .sin()) as f32
                    })
            })
            .collect();
        let clean = super::measure_nonharmonic_foldback_proxy(
            &clean,
            SAMPLE_RATE,
            fundamental_hz,
            24_000.0,
        );
        let folded = super::measure_nonharmonic_foldback_proxy(
            &folded,
            SAMPLE_RATE,
            fundamental_hz,
            24_000.0,
        );
        let on_grid = super::measure_nonharmonic_foldback_proxy(
            &on_grid,
            SAMPLE_RATE,
            fundamental_hz,
            24_000.0,
        );
        let expected_ratio = 3.0 * FOLDED_LEVEL.powi(2) / (1.0 + 0.3_f64.powi(2) + 0.2_f64.powi(2));
        let expected_db = super::ratio_db(expected_ratio);
        assert!(super::ratio_db(clean.ratio) <= -45.0, "{clean:?}");
        assert!(
            (super::ratio_db(folded.ratio) - expected_db).abs() <= 0.5,
            "expected_db={expected_db}, folded={folded:?}"
        );
        assert!(
            (super::ratio_db(on_grid.ratio) - super::ratio_db(clean.ratio)).abs() <= 1.0,
            "clean={clean:?}, on_grid={on_grid:?}"
        );
        assert!(folded.coverage <= 0.15, "{folded:?}");
    }

    #[test]
    fn overtone_magnitude_difference_rejects_gain_and_phase_but_detects_harmonics() {
        const FRAMES: usize = 131_072;
        const FUNDAMENTAL_HZ: f64 = 997.3;
        let reference: Vec<_> = (0..FRAMES)
            .map(|frame| {
                (std::f64::consts::TAU * FUNDAMENTAL_HZ * frame as f64 / f64::from(SAMPLE_RATE))
                    .sin() as f32
            })
            .collect();
        let gain_and_phase: Vec<_> = (0..FRAMES)
            .map(|frame| {
                (0.5 * (std::f64::consts::TAU * FUNDAMENTAL_HZ * frame as f64
                    / f64::from(SAMPLE_RATE)
                    + 0.8)
                    .sin()) as f32
            })
            .collect();
        let added_harmonic: Vec<_> = reference
            .iter()
            .enumerate()
            .map(|(frame, fundamental)| {
                *fundamental
                    + (0.1
                        * (std::f64::consts::TAU * 3.0 * FUNDAMENTAL_HZ * frame as f64
                            / f64::from(SAMPLE_RATE))
                        .sin()) as f32
            })
            .collect();

        let gain_phase_db = super::overtone_magnitude_difference_db(
            &gain_and_phase,
            &reference,
            SAMPLE_RATE,
            FUNDAMENTAL_HZ,
            24_000.0,
        );
        let added_harmonic_db = super::overtone_magnitude_difference_db(
            &added_harmonic,
            &reference,
            SAMPLE_RATE,
            FUNDAMENTAL_HZ,
            24_000.0,
        );
        assert!(gain_phase_db <= -75.0, "{gain_phase_db}");
        assert!(
            (-21.0..=-19.0).contains(&added_harmonic_db),
            "{added_harmonic_db}"
        );
    }
}
