//! Fixed-state bass instrument: a clean phase-locked body plus a separately
//! shaped upper-harmonic branch. All coefficient/state work is bounded and the
//! sample path allocates, locks, and performs no I/O.

use std::f32::consts::{LN_2, PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BassMatrixControls {
    pub body: f32,
    pub growl: f32,
    pub metal: f32,
    pub punch: f32,
    pub drive: f32,
    pub filter: f32,
    pub unstable: f32,
}

impl BassMatrixControls {
    pub const START: Self = Self {
        body: 0.66,
        growl: 0.18,
        metal: 0.08,
        punch: 0.42,
        drive: 0.20,
        filter: 0.48,
        unstable: 0.08,
    };

    pub fn from_macro_values(values: [f32; 8]) -> Self {
        Self {
            body: values[0],
            growl: values[1],
            metal: values[2],
            punch: values[3],
            // Slot four remains the historical Moj fifth macro. Physical
            // position five is host volume; the new model intentionally does
            // not let that level control alter timbre.
            drive: values[5],
            filter: values[6],
            unstable: values[7],
        }
        .bounded()
    }

    fn bounded(self) -> Self {
        Self {
            body: finite_unit(self.body),
            growl: finite_unit(self.growl),
            metal: finite_unit(self.metal),
            punch: finite_unit(self.punch),
            drive: finite_unit(self.drive),
            filter: finite_unit(self.filter),
            unstable: finite_unit(self.unstable),
        }
    }
}

#[derive(Debug)]
pub struct BassMatrixVoice {
    sample_rate: f32,
    frequency_hz: f32,
    performance_pitch: f32,
    controls: BassMatrixControls,
    velocity: f32,
    main_phase: f32,
    sub_phase: f32,
    metal_phase: f32,
    punch_envelope: f32,
    punch_decay: f32,
    low_state: f32,
    body_state: f32,
    body_alpha: f32,
    filter_alpha_low: f32,
    filter_alpha_high: f32,
    previous_grit: f32,
    feedback_state: f32,
    dc_input: f32,
    dc_output: f32,
    dc_coefficient: f32,
    random: u32,
    initial_random: u32,
}

impl BassMatrixVoice {
    pub fn new(sample_rate: f32, seed: u32) -> Option<Self> {
        if !sample_rate.is_finite() || !(8_000.0..=384_000.0).contains(&sample_rate) {
            return None;
        }
        Some(Self {
            sample_rate,
            frequency_hz: 110.0,
            performance_pitch: 1.0,
            controls: BassMatrixControls::START,
            velocity: 0.0,
            main_phase: 0.0,
            sub_phase: 0.0,
            metal_phase: 0.0,
            punch_envelope: 0.0,
            punch_decay: (-1.0 / (sample_rate * 0.048)).exp(),
            low_state: 0.0,
            body_state: 0.0,
            body_alpha: one_pole(sample_rate, 385.0),
            filter_alpha_low: one_pole(sample_rate, 154.0),
            filter_alpha_high: one_pole(sample_rate, 880.0),
            previous_grit: 0.0,
            feedback_state: 0.0,
            dc_input: 0.0,
            dc_output: 0.0,
            dc_coefficient: (-TAU * 8.0 / sample_rate).exp(),
            random: seed | 1,
            initial_random: seed | 1,
        })
    }

    pub fn set_controls(&mut self, controls: BassMatrixControls) {
        self.controls = controls.bounded();
    }

    pub(crate) fn set_pitch_ratio(&mut self, ratio: f32) {
        self.performance_pitch = ratio;
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.frequency_hz = 440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0);
        self.body_alpha = one_pole(
            self.sample_rate,
            (3.5 * self.frequency_hz).clamp(90.0, 520.0),
        );
        self.filter_alpha_low = one_pole(
            self.sample_rate,
            (1.4 * self.frequency_hz).clamp(70.0, 360.0),
        );
        self.filter_alpha_high = one_pole(
            self.sample_rate,
            (8.0 * self.frequency_hz).clamp(700.0, 7_500.0),
        );
        self.velocity = finite_unit(velocity);
        // Deliberate reset gives identical sub/main onset polarity and makes
        // the half-frequency oscillator truly phase locked per note.
        self.main_phase = 0.0;
        self.sub_phase = 0.0;
        self.metal_phase = 0.25;
        self.punch_envelope = 1.0;
        self.low_state = 0.0;
        self.body_state = 0.0;
        self.previous_grit = 0.0;
        self.feedback_state = 0.0;
        self.dc_input = 0.0;
        self.dc_output = 0.0;
        self.random = self.initial_random ^ u32::from(note).wrapping_mul(0x9e37_79b9);
    }

    #[inline]
    pub fn sample(&mut self) -> [f32; 2] {
        if self.velocity == 0.0 {
            return [0.0; 2];
        }
        let c = self.controls;
        let pitch_amount = 24.0 * c.punch * c.punch;
        let pitch_ratio = fast_positive_exp(pitch_amount * self.punch_envelope * (LN_2 / 12.0));
        let main_increment =
            (self.frequency_hz * pitch_ratio * self.performance_pitch / self.sample_rate).min(0.24);
        let sub_increment = 0.5 * main_increment;
        let metal_ratio = 1.5 + 0.914_213_54 * c.metal;
        let metal_increment = (main_increment * metal_ratio).min(0.31);

        self.random = xorshift(self.random);
        let random_bipolar = unit(self.random) * 2.0 - 1.0;
        let jitter = 1.0 + random_bipolar * 0.0025 * c.unstable * c.unstable;
        self.main_phase = wrap(self.main_phase + main_increment * jitter);
        self.sub_phase = wrap(self.sub_phase + sub_increment);
        self.metal_phase = wrap(self.metal_phase + metal_increment * jitter);
        self.punch_envelope *= self.punch_decay;

        let main_sine = (TAU * self.main_phase).sin();
        let sub = (TAU * self.sub_phase).sin();
        let modulator = (TAU * wrap(2.0 * self.main_phase + 0.17 * self.feedback_state)).sin();
        let pm_phase = wrap(self.main_phase + (0.02 + 0.43 * c.growl * c.growl) * modulator);
        let pm = (TAU * pm_phase).sin();
        let metal = (TAU * self.metal_phase).sin() * main_sine;

        // The body branch is always linear and centered. Increasing BODY
        // trades main fundamental for true f/2 weight without feeding the
        // nonlinear branch or its feedback.
        let body_mix = c.body * c.body;
        let body = (1.0 - 0.62 * body_mix) * main_sine + 0.78 * body_mix * sub;
        self.body_state += self.body_alpha * (body - self.body_state);

        let growl = pm - 0.55 * main_sine;
        let metallic = metal + 0.34 * (metal.abs() - 0.5);
        let feedback = self.feedback_state * (0.72 * c.unstable * c.unstable);
        let grit_input = 0.68 * c.growl * growl + 0.72 * c.metal * metallic + feedback;

        // Two midpoint substeps selectively oversample the nonlinear branch.
        // The clean body never passes through this waveshaper.
        let midpoint = 0.5 * (self.previous_grit + grit_input);
        let drive = 1.0 + 9.0 * c.drive * c.drive;
        let shaped_mid = shape(midpoint * drive, c.metal);
        let shaped_now = shape(grit_input * drive, c.metal);
        let mut grit = 0.5 * (shaped_mid + shaped_now) / (1.0 + 0.32 * drive);
        self.previous_grit = grit_input;

        let filter_position = c.filter * c.filter;
        let filter_alpha = self.filter_alpha_low
            + filter_position * (self.filter_alpha_high - self.filter_alpha_low);
        self.low_state += filter_alpha * (grit - self.low_state);
        let resonance = 0.62 * c.filter * c.filter;
        grit = self.low_state + resonance * (self.low_state - self.body_state);

        self.feedback_state = finite(grit).clamp(-0.8, 0.8);
        let combined = 0.72 * self.body_state + 0.78 * grit;
        let dc = combined - self.dc_input + self.dc_coefficient * self.dc_output;
        self.dc_input = combined;
        self.dc_output = finite(dc);
        let guarded = soft_guard(self.dc_output * self.velocity);

        // Only the upper branch moves; the body remains mono-compatible.
        let side = 0.12 * c.unstable * grit * (TAU * (self.metal_phase + 0.25)).sin();
        [soft_guard(guarded + side), soft_guard(guarded - side)]
    }

    pub fn reset(&mut self) {
        self.velocity = 0.0;
        self.feedback_state = 0.0;
        self.low_state = 0.0;
        self.body_state = 0.0;
        self.previous_grit = 0.0;
        self.dc_input = 0.0;
        self.dc_output = 0.0;
    }
}

#[inline]
fn shape(value: f32, asymmetry: f32) -> f32 {
    let biased = (value + 0.16 * asymmetry * value.abs()).clamp(-3.0, 3.0);
    let squared = biased * biased;
    biased * (27.0 + squared) / (27.0 + 9.0 * squared)
}

#[inline]
fn soft_guard(value: f32) -> f32 {
    let value = finite(value).clamp(-2.0, 2.0);
    (value * (1.0 - value.abs() * (1.0 / 6.0))).clamp(-0.94, 0.94)
}

#[inline]
fn one_pole(sample_rate: f32, cutoff: f32) -> f32 {
    1.0 - (-2.0 * PI * cutoff / sample_rate).exp()
}

/// Fifth-order exponential approximation over the pitch-punch domain
/// `0..=ln(4)`. This keeps the per-sample path free of transcendental pitch
/// conversion while remaining monotonic and within 0.3% of `exp` at the
/// intentionally extreme two-octave endpoint.
#[inline]
fn fast_positive_exp(value: f32) -> f32 {
    let value = value.clamp(0.0, 2.0 * LN_2);
    let polynomial = 1.0 / 120.0;
    let polynomial = 1.0 / 24.0 + value * polynomial;
    let polynomial = 1.0 / 6.0 + value * polynomial;
    let polynomial = 0.5 + value * polynomial;
    let polynomial = 1.0 + value * polynomial;
    1.0 + value * polynomial
}

#[inline]
fn wrap(value: f32) -> f32 {
    value - value.floor()
}

#[inline]
fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[inline]
fn finite_unit(value: f32) -> f32 {
    finite(value).clamp(0.0, 1.0)
}

#[inline]
fn xorshift(mut value: u32) -> u32 {
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

#[inline]
fn unit(value: u32) -> f32 {
    (value >> 8) as f32 * (1.0 / 16_777_216.0)
}
