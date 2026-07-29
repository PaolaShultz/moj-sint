use crate::model_d::{
    ModelDError,
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
const ALIAS_MEASUREMENT_FRAMES: usize = 32_768;
pub const MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS: usize = 4;
pub const MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB: f64 = 3.0;

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
    /// Non-harmonic 48 kHz spectral energy relative to target signal energy.
    pub target_residual_db: f64,
    /// Non-harmonic native 192 kHz energy in the same physical 0–24 kHz band.
    pub reference_floor_db: f64,
    /// Positive target residual power above the independently measured floor.
    pub excess_residual_db: f64,
    /// Raw time residual after fixed sinc resampling and the declared 7.75-host
    /// sample ladder-FIR alignment. It deliberately remains a non-acceptance
    /// diagnostic because it conflates transfer and phase with alias energy.
    pub transfer_conflated_residual_db: f64,
    pub target_fundamental_hz: f64,
    pub reference_fundamental_hz: f64,
    pub resolution_hz: f64,
    pub harmonic_mask_half_width_bins: usize,
    pub harmonic_mask_coverage: f64,
    pub nonlinear_harmonic_difference_db: f64,
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

pub fn measure_alias_residual(note: u8) -> Result<f64, ModelDError> {
    Ok(measure_alias_evidence(note)?.target_residual_db)
}

pub fn measure_alias_evidence(note: u8) -> Result<ModelDAliasEvidence, ModelDError> {
    if !(24..=108).contains(&note) {
        return Err(ModelDError::InvalidConfig);
    }
    let low_end = ALIAS_WARMUP_FRAMES_48K + ALIAS_MEASUREMENT_FRAMES;
    let analysis_radius = MODEL_D_ALIAS_ANALYSIS_FILTER_TAPS / 2;
    let high_end = low_end * ANALYSIS_RATE_FACTOR + analysis_radius + ANALYSIS_RATE_FACTOR;
    let low = render_alias_probe(note, CANONICAL_SAMPLE_RATE, low_end)?;
    let linear_low = render_alias_probe_mode(note, CANONICAL_SAMPLE_RATE, low_end, false)?;
    let high = render_alias_probe(
        note,
        CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
        high_end,
    )?;
    let target = &low[ALIAS_WARMUP_FRAMES_48K..low_end];
    let linear_target = &linear_low[ALIAS_WARMUP_FRAMES_48K..low_end];
    let native_reference_start = ALIAS_WARMUP_FRAMES_48K * ANALYSIS_RATE_FACTOR;
    let native_reference_end =
        native_reference_start + ALIAS_MEASUREMENT_FRAMES * ANALYSIS_RATE_FACTOR;
    let native_reference = &high[native_reference_start..native_reference_end];
    let reference =
        downsample_aligned_reference(&high, ALIAS_WARMUP_FRAMES_48K, ALIAS_MEASUREMENT_FRAMES);
    let fundamental_note = note.saturating_sub(12);
    let target_fundamental_hz = prepared_fundamental_hz(fundamental_note, CANONICAL_SAMPLE_RATE);
    let reference_fundamental_hz = prepared_fundamental_hz(
        fundamental_note,
        CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
    );
    let target_spectral = measure_spectral_foldback(
        target,
        CANONICAL_SAMPLE_RATE,
        target_fundamental_hz,
        24_000.0,
    );
    let reference_spectral = measure_spectral_foldback(
        native_reference,
        CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32,
        reference_fundamental_hz,
        24_000.0,
    );
    let excess_ratio = (target_spectral.ratio - reference_spectral.ratio).max(1.0e-24);
    Ok(ModelDAliasEvidence {
        target_residual_db: ratio_db(target_spectral.ratio),
        reference_floor_db: ratio_db(reference_spectral.ratio),
        excess_residual_db: ratio_db(excess_ratio),
        transfer_conflated_residual_db: fitted_residual_db(target, &reference),
        target_fundamental_hz,
        reference_fundamental_hz,
        resolution_hz: target_spectral.resolution_hz,
        harmonic_mask_half_width_bins: MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS,
        harmonic_mask_coverage: target_spectral.coverage,
        nonlinear_harmonic_difference_db: fitted_residual_db(target, linear_target),
    })
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
) -> Result<Vec<f32>, ModelDError> {
    render_alias_probe_mode(note, sample_rate, frame_count, true)
}

fn render_alias_probe_mode(
    note: u8,
    sample_rate: u32,
    frame_count: usize,
    nonlinear: bool,
) -> Result<Vec<f32>, ModelDError> {
    let diagnostics = ModelDDiagnostics {
        linear_mixer: !nonlinear,
        linear_ladder: !nonlinear,
        idealize_oscillators: true,
        disable_drift: true,
        disable_feedback: true,
    };
    let mut patch = ModelDPatch::bass();
    patch.source_levels = [0.66, 0.54, 0.255];
    patch.mixer_drive = 1.5;
    patch.ladder_drive = 1.5;
    let mut voice = ModelDVoice::new(sample_rate as f32, patch, diagnostics)?;
    voice.note_on(note, 0.82);
    Ok((0..frame_count).map(|_| voice.sample()).collect())
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
        sample_hash: hash_samples(samples),
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

fn hash_samples(samples: &[f32]) -> u64 {
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
struct SpectralFoldback {
    ratio: f64,
    coverage: f64,
    resolution_hz: f64,
}

fn measure_spectral_foldback(
    samples: &[f32],
    sample_rate: u32,
    fundamental_hz: f64,
    maximum_hz: f64,
) -> SpectralFoldback {
    assert!(samples.len().is_power_of_two());
    let sample_count = samples.len();
    let resolution_hz = f64::from(sample_rate) / sample_count as f64;
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

    let maximum_bin = (maximum_hz / resolution_hz).floor() as usize;
    let mut harmonic_mask = vec![false; maximum_bin + 1];
    let harmonic_count = (maximum_hz / fundamental_hz).floor() as usize;
    for harmonic in 1..=harmonic_count {
        let center = (fundamental_hz * harmonic as f64 / resolution_hz).round() as usize;
        let first = center
            .saturating_sub(MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS)
            .max(1);
        let last = (center + MODEL_D_ALIAS_HARMONIC_MASK_HALF_WIDTH_BINS).min(maximum_bin);
        harmonic_mask[first..=last].fill(true);
    }

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
    SpectralFoldback {
        ratio: (residual_energy / total_energy.max(1.0e-24)).max(1.0e-24),
        coverage: masked_bins as f64 / maximum_bin.max(1) as f64,
        resolution_hz,
    }
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
    let analysis_radius = (MODEL_D_ALIAS_ANALYSIS_FILTER_TAPS / 2) as isize;
    let ladder_alignment_high_rate = (ANALYSIS_RATE_FACTOR as f64 - 1.0)
        - (ANALYSIS_RATE_FACTOR as f64 - 1.0) * MODEL_D_LADDER_GROUP_DELAY_HOST;
    (0..output_frames)
        .map(|output_frame| {
            let low_frame = first_low_rate_frame + output_frame;
            let center =
                low_frame as f64 * ANALYSIS_RATE_FACTOR as f64 + ladder_alignment_high_rate;
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
                    / f64::from(CANONICAL_SAMPLE_RATE * ANALYSIS_RATE_FACTOR as u32);
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
    use super::{AuditionKind, measure_alias_evidence, measure_alias_residual, render_audition};

    const SAMPLE_RATE: u32 = 48_000;
    const MAX_PEAK_FOR_ONE_DB_HEADROOM: f64 = 0.891_250_938_133_745_6;

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
    fn steady_nonlinear_high_rate_alias_evidence_is_resolved_and_within_bounds() {
        for (note, maximum_db) in [(36, -45.0), (60, -45.0), (84, -35.0)] {
            let evidence = measure_alias_evidence(note).unwrap();
            let first = measure_alias_residual(note).unwrap();
            let second = measure_alias_residual(note).unwrap();
            assert_eq!(first, second, "note={note}");
            assert_eq!(first, evidence.target_residual_db, "note={note}");
            assert!(first.is_finite(), "note={note}, residual_db={first}");
            assert!(
                first <= maximum_db,
                "note={note}, residual_db={first}, maximum_db={maximum_db}, evidence={evidence:?}"
            );
            assert!(
                evidence.reference_floor_db
                    <= evidence.target_residual_db
                        - super::MODEL_D_ALIAS_MIN_REFERENCE_FLOOR_MARGIN_DB,
                "note={note}, evidence={evidence:?}"
            );
            assert!(
                evidence.excess_residual_db <= maximum_db,
                "note={note}, evidence={evidence:?}"
            );
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
            assert!(evidence.resolution_hz <= 1.5, "{evidence:?}");
            assert!(evidence.harmonic_mask_coverage < 0.50, "{evidence:?}");
            assert!(
                (-40.0..=-6.0).contains(&evidence.nonlinear_harmonic_difference_db),
                "note={note}, evidence={evidence:?}"
            );
        }
        assert!(measure_alias_residual(127).is_err());
    }

    #[test]
    fn spectral_mask_retains_a_synthetic_non_harmonic_folded_tone() {
        const FRAMES: usize = 32_768;
        const FUNDAMENTAL_HZ: f64 = 997.3;
        const FOLDED_HZ: f64 = 7_311.7;
        let clean: Vec<_> = (0..FRAMES)
            .map(|frame| {
                (std::f64::consts::TAU * FUNDAMENTAL_HZ * frame as f64 / f64::from(SAMPLE_RATE))
                    .sin() as f32
            })
            .collect();
        let folded: Vec<_> = clean
            .iter()
            .enumerate()
            .map(|(frame, clean)| {
                *clean
                    + 0.01
                        * (std::f64::consts::TAU * FOLDED_HZ * frame as f64
                            / f64::from(SAMPLE_RATE))
                        .sin() as f32
            })
            .collect();
        let clean = super::measure_spectral_foldback(&clean, SAMPLE_RATE, FUNDAMENTAL_HZ, 24_000.0);
        let folded =
            super::measure_spectral_foldback(&folded, SAMPLE_RATE, FUNDAMENTAL_HZ, 24_000.0);
        assert!(super::ratio_db(clean.ratio) <= -75.0, "{clean:?}");
        assert!(
            (-43.0..=-37.0).contains(&super::ratio_db(folded.ratio)),
            "{folded:?}"
        );
        assert!(folded.coverage < 0.50, "{folded:?}");
    }
}
