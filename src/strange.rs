//! Strange Oscillator: the third live experimental synthesis model.

use crate::dsp::oscillator::{BandlimitedOscillator, OscillatorMethod, SineOscillator};
use thiserror::Error;

const SCANNED_POINTS: usize = 16;
const STOCHASTIC_POINTS: usize = 8;
const LFO_RATES_HZ: [f32; 17] = [
    0.05,
    0.076_996_33,
    0.118_568_69,
    0.182_587_07,
    0.281_170_67,
    0.432_982_18,
    0.666_760_74,
    1.026_762_5,
    1.581_138_8,
    2.434_837_6,
    3.749_471,
    5.773_91,
    8.891_397,
    13.692_098,
    21.084_826,
    32.469_08,
    50.0,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrangeType {
    Triangle,
    Saw,
    Pulse,
    ModulatedResonator,
    DeformedLoop,
    StochasticBreakpoints,
    ScannedString,
    RegisterMachine,
}

impl StrangeType {
    pub const ALL: [Self; 8] = [
        Self::Triangle,
        Self::Saw,
        Self::Pulse,
        Self::ModulatedResonator,
        Self::DeformedLoop,
        Self::StochasticBreakpoints,
        Self::ScannedString,
        Self::RegisterMachine,
    ];

    pub fn from_normalized(value: f32) -> Self {
        let value = if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self::ALL[(value * 7.0).round() as usize]
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Triangle => "triangle",
            Self::Saw => "saw",
            Self::Pulse => "pulse",
            Self::ModulatedResonator => "modulated-resonator",
            Self::DeformedLoop => "deformed-loop",
            Self::StochasticBreakpoints => "stochastic-breakpoints",
            Self::ScannedString => "scanned-string",
            Self::RegisterMachine => "register-machine",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrangeControls {
    pub form: f32,
    pub warp: f32,
    pub couple: f32,
    pub motion: f32,
    pub chaos: f32,
    pub color: f32,
    pub space: f32,
}

impl StrangeControls {
    pub const MIDPOINT: Self = Self {
        form: 0.5,
        warp: 0.5,
        couple: 0.5,
        motion: 0.5,
        chaos: 0.5,
        color: 0.5,
        space: 0.5,
    };

    pub fn with(mut self, slot: usize, value: f32) -> Self {
        let value = sanitize(value);
        match slot {
            0 => self.form = value,
            1 => self.warp = value,
            2 => self.couple = value,
            3 => self.motion = value,
            4 => self.chaos = value,
            5 => self.color = value,
            6 => self.space = value,
            _ => {}
        }
        self
    }

    pub fn lfo_rate_hz(self) -> f32 {
        let position = sanitize(self.motion) * (LFO_RATES_HZ.len() - 1) as f32;
        let lower = (position as usize).min(LFO_RATES_HZ.len() - 2);
        let fraction = position - lower as f32;
        LFO_RATES_HZ[lower] + fraction * (LFO_RATES_HZ[lower + 1] - LFO_RATES_HZ[lower])
    }

    fn sanitized(self) -> Self {
        Self {
            form: sanitize(self.form),
            warp: sanitize(self.warp),
            couple: sanitize(self.couple),
            motion: sanitize(self.motion),
            chaos: sanitize(self.chaos),
            color: sanitize(self.color),
            space: sanitize(self.space),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StrangeFrame {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum StrangeError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("frequency must be finite, positive, and below Nyquist")]
    InvalidFrequency,
}

#[derive(Debug)]
pub struct StrangeVoice {
    state: StrangeState,
    structural: StructuralState,
    tone_left: OnePole,
    tone_right: OnePole,
    tone_second_left: OnePole,
    tone_second_right: OnePole,
    dc_left: DcBlocker,
    dc_right: DcBlocker,
}

#[derive(Debug)]
pub struct StrangeInstrument {
    current_type: StrangeType,
    current: StrangeVoice,
    incoming: Option<(StrangeType, StrangeVoice)>,
    transition_sample: u32,
    transition_samples: u32,
    sample_rate: f32,
    frequency_hz: f32,
    seed: u32,
    controls: StrangeControls,
}

impl StrangeInstrument {
    pub fn new(
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
        type_normalized: f32,
        controls: StrangeControls,
    ) -> Result<Self, StrangeError> {
        let current_type = StrangeType::from_normalized(type_normalized);
        let current = StrangeVoice::new(current_type, sample_rate, frequency_hz, seed, controls)?;
        Ok(Self {
            current_type,
            current,
            incoming: None,
            transition_sample: 0,
            transition_samples: (0.01 * sample_rate).round().max(64.0) as u32,
            sample_rate,
            frequency_hz,
            seed,
            controls: controls.sanitized(),
        })
    }

    pub fn set_type_normalized(&mut self, value: f32) -> Result<StrangeType, StrangeError> {
        let kind = StrangeType::from_normalized(value);
        self.set_type(kind)?;
        Ok(kind)
    }

    pub fn set_type(&mut self, kind: StrangeType) -> Result<(), StrangeError> {
        if kind == self.current_type && self.incoming.is_none() {
            return Ok(());
        }
        if self
            .incoming
            .as_ref()
            .is_some_and(|(incoming, _)| *incoming == kind)
        {
            return Ok(());
        }
        let voice = StrangeVoice::new(
            kind,
            self.sample_rate,
            self.frequency_hz,
            self.seed ^ ((kind as u32 + 1) * 0x1020_3041),
            self.controls,
        )?;
        self.incoming = Some((kind, voice));
        self.transition_sample = 0;
        Ok(())
    }

    pub fn set_controls(&mut self, controls: StrangeControls) {
        let controls = controls.sanitized();
        self.controls = controls;
        self.current.set_controls(controls);
        if let Some((_, incoming)) = self.incoming.as_mut() {
            incoming.set_controls(controls);
        }
    }

    pub fn reset(&mut self) {
        self.current.reset();
        self.incoming = None;
        self.transition_sample = 0;
    }

    #[inline]
    pub fn sample(&mut self) -> StrangeFrame {
        let current = self.current.sample();
        let Some((_, incoming)) = self.incoming.as_mut() else {
            return current;
        };
        let next = incoming.sample();
        let mix = self.transition_sample as f32 / self.transition_samples as f32;
        let frame = StrangeFrame {
            left: current.left + mix * (next.left - current.left),
            right: current.right + mix * (next.right - current.right),
        };
        self.transition_sample += 1;
        if self.transition_sample >= self.transition_samples {
            let (kind, voice) = self.incoming.take().expect("incoming voice exists");
            self.current_type = kind;
            self.current = voice;
            self.transition_sample = 0;
        }
        frame
    }

    pub const fn current_type(&self) -> StrangeType {
        self.current_type
    }

    pub const fn is_transitioning(&self) -> bool {
        self.incoming.is_some()
    }
}

impl StrangeVoice {
    pub fn new(
        kind: StrangeType,
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
        controls: StrangeControls,
    ) -> Result<Self, StrangeError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(StrangeError::InvalidSampleRate);
        }
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 || frequency_hz >= 0.5 * sample_rate {
            return Err(StrangeError::InvalidFrequency);
        }
        let controls = controls.sanitized();
        let state = match kind {
            StrangeType::Triangle | StrangeType::Saw | StrangeType::Pulse => StrangeState::Classic(
                ClassicState::new(kind, sample_rate, frequency_hz, seed, controls)?,
            ),
            StrangeType::ModulatedResonator => StrangeState::Modulated(ModulatedState::new(
                sample_rate,
                frequency_hz,
                seed,
                controls,
            )?),
            StrangeType::DeformedLoop => {
                StrangeState::Loop(LoopState::new(sample_rate, frequency_hz, seed, controls))
            }
            StrangeType::StochasticBreakpoints => StrangeState::Stochastic(StochasticState::new(
                sample_rate,
                frequency_hz,
                seed,
                controls,
            )),
            StrangeType::ScannedString => {
                StrangeState::Scanned(ScannedState::new(sample_rate, frequency_hz, seed, controls))
            }
            StrangeType::RegisterMachine => StrangeState::Register(RegisterState::new(
                sample_rate,
                frequency_hz,
                seed,
                controls,
            )),
        };
        let cutoff_hz = 60.0 + 18_000.0 * controls.color * controls.color * controls.color;
        let tone_coefficient = 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp();
        Ok(Self {
            state,
            structural: StructuralState::new(
                sample_rate,
                frequency_hz,
                seed ^ 0xb529_7a4d,
                controls,
            )?,
            tone_left: OnePole::new(tone_coefficient),
            tone_right: OnePole::new((tone_coefficient * 0.997).clamp(0.001, 0.95)),
            tone_second_left: OnePole::new(tone_coefficient),
            tone_second_right: OnePole::new((tone_coefficient * 0.997).clamp(0.001, 0.95)),
            dc_left: DcBlocker::new(sample_rate, 7.0),
            dc_right: DcBlocker::new(sample_rate, 7.0),
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StrangeFrame {
        let raw = self.structural.process(self.state.sample());
        let left = self.dc_left.sample(
            self.tone_second_left
                .sample(self.tone_left.sample(raw.left)),
        );
        let right = self.dc_right.sample(
            self.tone_second_right
                .sample(self.tone_right.sample(raw.right)),
        );
        StrangeFrame {
            left: (0.78 * left).clamp(-1.0, 1.0),
            right: (0.78 * right).clamp(-1.0, 1.0),
        }
    }

    pub fn reset(&mut self) {
        self.state.reset();
        self.structural.reset();
        self.tone_left.reset();
        self.tone_right.reset();
        self.tone_second_left.reset();
        self.tone_second_right.reset();
        self.dc_left.reset();
        self.dc_right.reset();
    }

    fn set_controls(&mut self, controls: StrangeControls) {
        self.state.set_controls(controls);
        self.structural.set_controls(controls);
        let coefficient = tone_coefficient(self.structural.sample_rate, controls.color);
        self.tone_left.set_coefficient(coefficient);
        self.tone_right
            .set_coefficient((coefficient * 0.997).clamp(0.001, 0.95));
        self.tone_second_left.set_coefficient(coefficient);
        self.tone_second_right
            .set_coefficient((coefficient * 0.997).clamp(0.001, 0.95));
    }
}

#[derive(Debug)]
struct StructuralState {
    coupler_left: SineOscillator,
    coupler_right: SineOscillator,
    color_left: SineOscillator,
    color_right: SineOscillator,
    bright_left: SineOscillator,
    bright_right: SineOscillator,
    motion: SineOscillator,
    motion_step: u8,
    sample_rate: f32,
    rng: XorShift,
    seed: u32,
    held_cycle: f32,
    previous_motion: f32,
    controls: StrangeControls,
}

impl StructuralState {
    fn new(
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
        controls: StrangeControls,
    ) -> Result<Self, StrangeError> {
        let mut coupler_left =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut coupler_right =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        let ratio = 1.25 + 4.75 * controls.form;
        coupler_left.set_frequency((frequency_hz * ratio).min(0.42 * sample_rate));
        coupler_right.set_frequency((frequency_hz * (ratio + 0.07)).min(0.42 * sample_rate));
        let mut color_left =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut color_right =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        color_left.set_frequency(frequency_hz);
        color_right.set_frequency(frequency_hz);
        let mut bright_left =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut bright_right =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        bright_left.set_frequency((5.0 * frequency_hz).min(0.42 * sample_rate));
        bright_right.set_frequency((5.03 * frequency_hz).min(0.42 * sample_rate));
        let mut motion =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        motion.set_frequency(controls.lfo_rate_hz());
        Ok(Self {
            coupler_left,
            coupler_right,
            color_left,
            color_right,
            bright_left,
            bright_right,
            motion,
            motion_step: (controls.motion * 128.0).round() as u8,
            sample_rate,
            rng: XorShift::new(seed),
            seed,
            held_cycle: 0.0,
            previous_motion: 0.0,
            controls,
        })
    }

    #[inline]
    fn process(&mut self, frame: StrangeFrame) -> StrangeFrame {
        let motion = self.motion.sample();
        if self.previous_motion <= 0.0 && motion > 0.0 {
            self.held_cycle = self.rng.bipolar();
        }
        self.previous_motion = motion;

        let warp = ((self.controls.warp - 0.45) / 0.55).clamp(0.0, 1.0);
        let left = mix(frame.left, fifth_harmonic(frame.left), warp);
        let right = mix(frame.right, fifth_harmonic(frame.right), warp);

        let couple = ((self.controls.couple - 0.45) / 0.55).clamp(0.0, 1.0);
        let ring_left = (1.55 * left * self.coupler_left.sample()).clamp(-1.0, 1.0);
        let ring_right = (1.55 * right * self.coupler_right.sample()).clamp(-1.0, 1.0);
        let left = mix(left, ring_left, couple);
        let right = mix(right, ring_right, couple);

        let motion_depth = 0.12 + 0.82 * (2.0 * self.controls.motion - 1.0).abs();
        let cyclic_envelope = 0.04 + 0.96 * (0.5 + 0.5 * motion);
        let motion_gain = mix(1.0, cyclic_envelope, motion_depth);

        let chaos = ((self.controls.chaos - 0.5) / 0.4).clamp(0.0, 1.0);
        // At the upper end CHAOS changes the machine's cycle grammar: complete
        // modulation cycles are either admitted or nearly dropped.  A binary
        // held decision stays unmistakable even when the selected source is
        // already stochastic, while the midpoint remains the stable baseline.
        let irregular_gain = if self.held_cycle >= 0.0 { 1.0 } else { 0.04 };
        let gain = motion_gain * mix(1.0, irregular_gain, chaos);
        let left = gain * left;
        let right = gain * right;

        let dark = ((0.5 - self.controls.color) * 2.0).max(0.0);
        let bright = ((self.controls.color - 0.5) * 2.0).max(0.0);
        let left = mix(left, 0.72 * self.color_left.sample(), 0.9 * dark);
        let right = mix(right, 0.72 * self.color_right.sample(), 0.9 * dark);
        let left = mix(left, 0.72 * self.bright_left.sample(), 0.9 * bright);
        let right = mix(right, 0.72 * self.bright_right.sample(), 0.9 * bright);

        let mid = 0.5 * (left + right);
        let side = 0.5 * (left - right);
        let width = 0.05 + 2.4 * self.controls.space * self.controls.space;
        StrangeFrame {
            left: (mid + width * side).clamp(-1.0, 1.0),
            right: (mid - width * side).clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.coupler_left.reset();
        self.coupler_right.reset();
        self.color_left.reset();
        self.color_right.reset();
        self.bright_left.reset();
        self.bright_right.reset();
        self.motion.reset();
        self.rng = XorShift::new(self.seed);
        self.held_cycle = 0.0;
        self.previous_motion = 0.0;
    }

    fn set_controls(&mut self, controls: StrangeControls) {
        let motion_step = (controls.motion * 128.0).round() as u8;
        if motion_step != self.motion_step {
            let mut quantized = controls;
            quantized.motion = f32::from(motion_step) / 128.0;
            self.motion.set_frequency(quantized.lfo_rate_hz());
            self.motion_step = motion_step;
        }
        self.controls = controls;
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum StrangeState {
    Classic(ClassicState),
    Modulated(ModulatedState),
    Loop(LoopState),
    Stochastic(StochasticState),
    Scanned(ScannedState),
    Register(RegisterState),
}

impl StrangeState {
    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        match self {
            Self::Classic(state) => state.sample(),
            Self::Modulated(state) => state.sample(),
            Self::Loop(state) => state.sample(),
            Self::Stochastic(state) => state.sample(),
            Self::Scanned(state) => state.sample(),
            Self::Register(state) => state.sample(),
        }
    }

    fn reset(&mut self) {
        match self {
            Self::Classic(state) => state.reset(),
            Self::Modulated(state) => state.reset(),
            Self::Loop(state) => state.reset(),
            Self::Stochastic(state) => state.reset(),
            Self::Scanned(state) => state.reset(),
            Self::Register(state) => state.reset(),
        }
    }

    fn set_controls(&mut self, controls: StrangeControls) {
        match self {
            Self::Classic(state) => state.controls = controls,
            Self::Modulated(state) => state.controls = controls,
            Self::Loop(state) => state.controls = controls,
            Self::Stochastic(state) => state.controls = controls,
            Self::Scanned(state) => state.controls = controls,
            Self::Register(state) => state.controls = controls,
        }
    }
}

#[derive(Debug)]
struct ClassicState {
    kind: StrangeType,
    saw_left: BandlimitedOscillator,
    saw_right: BandlimitedOscillator,
    phase_left: f32,
    phase_right: f32,
    increment_left: f32,
    increment_right: f32,
    motion_phase: f32,
    motion_increment: f32,
    rng: XorShift,
    seed: u32,
    held_cycle: f32,
    previous_movement: f32,
    controls: StrangeControls,
}

impl ClassicState {
    fn new(
        kind: StrangeType,
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
        controls: StrangeControls,
    ) -> Result<Self, StrangeError> {
        let detune = 1.0 + 0.006 * controls.space;
        let mut saw_left =
            BandlimitedOscillator::new(sample_rate, OscillatorMethod::IntegratedWavetable)
                .map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut saw_right =
            BandlimitedOscillator::new(sample_rate, OscillatorMethod::IntegratedWavetable)
                .map_err(|_| StrangeError::InvalidSampleRate)?;
        saw_left.set_frequency(frequency_hz / detune.sqrt());
        saw_right.set_frequency(frequency_hz * detune.sqrt());
        Ok(Self {
            kind,
            saw_left,
            saw_right,
            phase_left: 0.0,
            phase_right: 0.0,
            increment_left: frequency_hz / (sample_rate * detune.sqrt()),
            increment_right: frequency_hz * detune.sqrt() / sample_rate,
            motion_phase: 0.0,
            motion_increment: controls.lfo_rate_hz() / sample_rate,
            rng: XorShift::new(seed),
            seed,
            held_cycle: 0.0,
            previous_movement: -1.0,
            controls,
        })
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        let movement = triangle(self.motion_phase);
        self.motion_phase = advance(self.motion_phase, self.motion_increment);
        if self.previous_movement <= 0.0 && movement > 0.0 {
            self.held_cycle = self.rng.bipolar();
        }
        self.previous_movement = movement;
        let chaos_amount = ((self.controls.chaos - 0.5) * 2.0).max(0.0);
        let irregular = chaos_amount * self.held_cycle;
        let left_motion = (movement + 0.55 * irregular).clamp(-1.0, 1.0);
        let right_motion = (mix(movement, -movement, 0.65 * self.controls.space)
            - 0.45 * irregular)
            .clamp(-1.0, 1.0);
        let left = self.channel(true, left_motion);
        let right = self.channel(false, right_motion);
        StrangeFrame { left, right }
    }

    #[inline]
    fn channel(&mut self, left: bool, movement: f32) -> f32 {
        let base = match self.kind {
            StrangeType::Triangle => {
                let phase = if left {
                    let phase = self.phase_left;
                    self.phase_left = advance(self.phase_left, self.increment_left);
                    phase
                } else {
                    let phase = self.phase_right;
                    self.phase_right = advance(self.phase_right, self.increment_right);
                    phase
                };
                let moving_form = (self.controls.form + 0.28 * movement).clamp(0.0, 1.0);
                let tri = triangle(warp_phase(phase, moving_form));
                let square = if phase < 0.5 { -1.0 } else { 1.0 };
                tri + 0.28 * moving_form * (square - tri)
            }
            StrangeType::Saw => {
                let shape = (0.42 * self.controls.form + 0.18 * movement).clamp(0.0, 0.58);
                if left {
                    self.saw_left.sample(shape)
                } else {
                    self.saw_right.sample(shape)
                }
            }
            StrangeType::Pulse => {
                let shape = (0.58 + 0.42 * self.controls.form + 0.16 * movement).clamp(0.42, 1.0);
                if left {
                    self.saw_left.sample(shape)
                } else {
                    self.saw_right.sample(shape)
                }
            }
            _ => 0.0,
        };
        let warp = 1.4 * (self.controls.warp - 0.5);
        let warped = base + warp * (base - base * base * base);
        let couple = 0.68 * ((self.controls.couple - 0.5) * 2.0).max(0.0);
        let coupled = (1.0 - couple) * warped + couple * warped * movement_sign(base);
        let envelope = 0.62 + 0.38 * (0.5 + 0.5 * movement);
        (0.65 * envelope * coupled).clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        self.saw_left.reset();
        self.saw_right.reset();
        self.phase_left = 0.0;
        self.phase_right = 0.0;
        self.motion_phase = 0.0;
        self.rng = XorShift::new(self.seed);
        self.held_cycle = 0.0;
        self.previous_movement = -1.0;
    }
}

#[derive(Debug)]
struct ModulatedState {
    carrier_left: BandlimitedOscillator,
    carrier_right: BandlimitedOscillator,
    modal_left: SineOscillator,
    modal_right: SineOscillator,
    lfo: SineOscillator,
    rng: XorShift,
    seed: u32,
    held_cycle: f32,
    previous_lfo: f32,
    controls: StrangeControls,
}

impl ModulatedState {
    fn new(
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
        controls: StrangeControls,
    ) -> Result<Self, StrangeError> {
        let detune = 1.0 + 0.004 * controls.space;
        let mut carrier_left =
            BandlimitedOscillator::new(sample_rate, OscillatorMethod::IntegratedWavetable)
                .map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut carrier_right =
            BandlimitedOscillator::new(sample_rate, OscillatorMethod::IntegratedWavetable)
                .map_err(|_| StrangeError::InvalidSampleRate)?;
        carrier_left.set_frequency(frequency_hz / detune.sqrt());
        carrier_right.set_frequency(frequency_hz * detune.sqrt());
        let modal_ratio = 1.5 + 3.5 * controls.form;
        let mut modal_left =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        let mut modal_right =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        modal_left.set_frequency(frequency_hz * modal_ratio / detune.sqrt());
        modal_right.set_frequency(frequency_hz * modal_ratio * detune.sqrt());
        let mut lfo =
            SineOscillator::new(sample_rate).map_err(|_| StrangeError::InvalidSampleRate)?;
        lfo.set_frequency(controls.lfo_rate_hz());
        Ok(Self {
            carrier_left,
            carrier_right,
            modal_left,
            modal_right,
            lfo,
            rng: XorShift::new(seed),
            seed,
            held_cycle: 0.0,
            previous_lfo: 0.0,
            controls,
        })
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        let lfo = self.lfo.sample();
        if self.previous_lfo <= 0.0 && lfo > 0.0 {
            self.held_cycle = self.rng.bipolar();
        }
        self.previous_lfo = lfo;
        let chaos = ((self.controls.chaos - 0.5) * 2.0).max(0.0) * self.held_cycle;
        let left_lfo = (lfo + 0.72 * chaos).clamp(-1.0, 1.0);
        let right_lfo =
            (mix(lfo, -lfo, 0.72 * self.controls.space) - 0.58 * chaos).clamp(-1.0, 1.0);
        let depth = 0.88 * self.controls.couple;
        let envelope_left = mix(1.0, 0.12 + 0.88 * (0.5 + 0.5 * left_lfo), depth);
        let envelope_right = mix(1.0, 0.12 + 0.88 * (0.5 + 0.5 * right_lfo), depth);
        let shape_left = (0.12 + 0.7 * self.controls.form + 0.1 * depth * left_lfo + 0.22 * chaos)
            .clamp(0.0, 1.0);
        let shape_right = (0.12 + 0.7 * self.controls.form + 0.1 * depth * right_lfo
            - 0.18 * chaos)
            .clamp(0.0, 1.0);
        let carrier_left = self.carrier_left.sample(shape_left);
        let carrier_right = self.carrier_right.sample(shape_right);
        let warp = 1.5 * (self.controls.warp - 0.5);
        let carrier_left = carrier_left + warp * (carrier_left - carrier_left.powi(3));
        let carrier_right = carrier_right + warp * (carrier_right - carrier_right.powi(3));
        let modal_left = self.modal_left.sample();
        let modal_right = self.modal_right.sample();
        let modal_mix_left =
            (0.12 + 0.4 * depth * (0.5 + 0.5 * left_lfo) + 0.24 * chaos).clamp(0.0, 0.8);
        let modal_mix_right =
            (0.12 + 0.4 * depth * (0.5 + 0.5 * right_lfo) - 0.2 * chaos).clamp(0.0, 0.8);
        StrangeFrame {
            left: (0.58 * envelope_left * (carrier_left + modal_mix_left * modal_left))
                .clamp(-1.0, 1.0),
            right: (0.58 * envelope_right * (carrier_right + modal_mix_right * modal_right))
                .clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.carrier_left.reset();
        self.carrier_right.reset();
        self.modal_left.reset();
        self.modal_right.reset();
        self.lfo.reset();
        self.rng = XorShift::new(self.seed);
        self.held_cycle = 0.0;
        self.previous_lfo = 0.0;
    }
}

#[derive(Debug)]
struct LoopState {
    phase: f32,
    increment: f32,
    movement_phase: f32,
    movement_increment: f32,
    previous: f32,
    rng: XorShift,
    seed: u32,
    controls: StrangeControls,
    y_points: [f32; 8],
    x_points: [f32; 8],
}

impl LoopState {
    fn new(sample_rate: f32, frequency_hz: f32, seed: u32, controls: StrangeControls) -> Self {
        let irregular_y = [0.0, 0.92, 0.25, 0.78, -0.36, -0.96, -0.14, -0.72];
        let regular_y = [0.0, 0.71, 1.0, 0.71, 0.0, -0.71, -1.0, -0.71];
        let irregular_x = [1.0, 0.18, -0.72, -0.12, -1.0, -0.31, 0.66, 0.38];
        let regular_x = [1.0, 0.71, 0.0, -0.71, -1.0, -0.71, 0.0, 0.71];
        Self {
            phase: 0.0,
            increment: frequency_hz / sample_rate,
            movement_phase: 0.0,
            movement_increment: (0.04 + 2.5 * controls.motion * controls.motion) / sample_rate,
            previous: 0.0,
            rng: XorShift::new(seed),
            seed,
            controls,
            y_points: std::array::from_fn(|index| {
                mix(regular_y[index], irregular_y[index], controls.form)
            }),
            x_points: std::array::from_fn(|index| {
                mix(regular_x[index], irregular_x[index], controls.form)
            }),
        }
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        let movement = triangle(self.movement_phase);
        self.movement_phase = advance(self.movement_phase, self.movement_increment);
        let random = self.rng.bipolar();
        let chaos_amount = ((self.controls.chaos - 0.5) * 2.0).max(0.0);
        let perturb =
            (2.0 * self.controls.warp - 1.0) * self.previous + 0.35 * chaos_amount * random;
        let step = self.increment * (1.0 + 0.62 * perturb).clamp(0.2, 1.8);
        self.phase = advance(self.phase, step);
        let y = closed_cubic(&self.y_points, self.phase);
        let x = closed_cubic(
            &self.x_points,
            advance(self.phase, 0.125 * self.controls.couple),
        );
        self.previous = (0.88 * self.previous + 0.12 * y).clamp(-1.0, 1.0);
        let breath = 1.0 + 0.18 * movement * (2.0 * self.controls.motion - 1.0);
        let center = breath * mix(y, 0.72 * y + 0.28 * x, self.controls.couple);
        let side = 0.46 * self.controls.space * (x - y);
        StrangeFrame {
            left: (0.62 * (center + side)).clamp(-1.0, 1.0),
            right: (0.62 * (center - side)).clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.movement_phase = 0.0;
        self.previous = 0.0;
        self.rng = XorShift::new(self.seed);
    }
}

#[derive(Debug)]
struct StochasticState {
    phase: f32,
    increment: f32,
    amplitudes_left: [f32; STOCHASTIC_POINTS],
    amplitudes_right: [f32; STOCHASTIC_POINTS],
    initial_left: [f32; STOCHASTIC_POINTS],
    initial_right: [f32; STOCHASTIC_POINTS],
    rng_left: XorShift,
    rng_right: XorShift,
    seed: u32,
    cycles: u8,
    controls: StrangeControls,
}

impl StochasticState {
    fn new(sample_rate: f32, frequency_hz: f32, seed: u32, controls: StrangeControls) -> Self {
        let initial_left = std::array::from_fn(|index| {
            let phase = index as f32 / STOCHASTIC_POINTS as f32;
            mix(
                triangle(phase),
                if index % 2 == 0 { -0.8 } else { 0.8 },
                controls.form,
            )
        });
        let initial_right = initial_left;
        Self {
            phase: 0.0,
            increment: frequency_hz / sample_rate,
            amplitudes_left: initial_left,
            amplitudes_right: initial_right,
            initial_left,
            initial_right,
            rng_left: XorShift::new(seed),
            rng_right: XorShift::new(seed ^ 0x6c8e_9cf5),
            seed,
            cycles: 0,
            controls,
        }
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        let old_phase = self.phase;
        self.phase = advance(self.phase, self.increment);
        if self.phase < old_phase {
            self.cycles = self.cycles.wrapping_add(1);
            let interval = 1 + (5.0 * (1.0 - self.controls.motion)).round() as u8;
            if self.cycles % interval == 0 {
                update_breakpoints(&mut self.amplitudes_left, &mut self.rng_left, self.controls);
                update_breakpoints(
                    &mut self.amplitudes_right,
                    &mut self.rng_right,
                    self.controls,
                );
            }
        }
        let left = breakpoint_sample(&self.amplitudes_left, self.phase, self.controls.warp);
        let independent = breakpoint_sample(
            &self.amplitudes_right,
            advance(self.phase, 0.03 * self.controls.space),
            self.controls.warp,
        );
        let right = mix(left, independent, self.controls.space);
        StrangeFrame {
            left: (0.7 * left).clamp(-1.0, 1.0),
            right: (0.7 * right).clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.amplitudes_left = self.initial_left;
        self.amplitudes_right = self.initial_right;
        self.rng_left = XorShift::new(self.seed);
        self.rng_right = XorShift::new(self.seed ^ 0x6c8e_9cf5);
        self.cycles = 0;
    }
}

fn update_breakpoints(
    amplitudes: &mut [f32; STOCHASTIC_POINTS],
    rng: &mut XorShift,
    controls: StrangeControls,
) {
    let previous = *amplitudes;
    let step = 0.008 + 0.24 * controls.chaos * controls.chaos;
    for index in 0..STOCHASTIC_POINTS {
        let neighbours = 0.5
            * (previous[(index + STOCHASTIC_POINTS - 1) % STOCHASTIC_POINTS]
                + previous[(index + 1) % STOCHASTIC_POINTS]);
        let walked = previous[index] + step * rng.bipolar();
        amplitudes[index] = mix(walked, neighbours, 0.18 * controls.couple).clamp(-1.0, 1.0);
    }
}

#[inline]
fn breakpoint_sample(points: &[f32; STOCHASTIC_POINTS], phase: f32, warp: f32) -> f32 {
    let position = phase * STOCHASTIC_POINTS as f32;
    let index = position as usize % STOCHASTIC_POINTS;
    let next = (index + 1) % STOCHASTIC_POINTS;
    let linear = position - position.floor();
    let smooth = linear * linear * (3.0 - 2.0 * linear);
    mix(points[index], points[next], mix(linear, smooth, warp))
}

#[derive(Debug)]
struct ScannedState {
    position: [f32; SCANNED_POINTS],
    velocity: [f32; SCANNED_POINTS],
    initial_position: [f32; SCANNED_POINTS],
    phase: f32,
    phase_increment: f32,
    haptic_phase: f32,
    haptic_increment: f32,
    rng: XorShift,
    seed: u32,
    controls: StrangeControls,
}

impl ScannedState {
    fn new(sample_rate: f32, frequency_hz: f32, seed: u32, controls: StrangeControls) -> Self {
        let initial_position = std::array::from_fn(|index| {
            let phase = index as f32 / SCANNED_POINTS as f32;
            let fundamental = triangle(phase);
            let knot = if index % 3 == 0 { 0.7 } else { -0.25 };
            mix(fundamental, knot, controls.form)
        });
        Self {
            position: initial_position,
            velocity: [0.0; SCANNED_POINTS],
            initial_position,
            phase: 0.0,
            phase_increment: frequency_hz / sample_rate,
            haptic_phase: 0.0,
            haptic_increment: (0.25 + 12.0 * controls.motion * controls.motion) / sample_rate,
            rng: XorShift::new(seed),
            seed,
            controls,
        }
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        self.phase = advance(self.phase, self.phase_increment);
        let old_haptic = self.haptic_phase;
        self.haptic_phase = advance(self.haptic_phase, self.haptic_increment);
        if self.haptic_phase < old_haptic {
            self.update_string();
        }
        let warped = warp_phase(self.phase, self.controls.warp);
        let left = scan_points(&self.position, warped);
        let right_scan = advance(warped, 0.22 * self.controls.space);
        let right = mix(
            left,
            scan_points(&self.position, right_scan),
            self.controls.space,
        );
        StrangeFrame {
            left: (0.72 * left).clamp(-1.0, 1.0),
            right: (0.72 * right).clamp(-1.0, 1.0),
        }
    }

    fn update_string(&mut self) {
        let old_position = self.position;
        let stiffness = 0.035 + 0.21 * self.controls.couple;
        let damping = 0.91 - 0.20 * self.controls.color;
        for index in 0..SCANNED_POINTS {
            let left = old_position[(index + SCANNED_POINTS - 1) % SCANNED_POINTS];
            let right = old_position[(index + 1) % SCANNED_POINTS];
            let spring = left + right - 2.0 * old_position[index];
            let force = 0.045 * self.controls.chaos * self.rng.bipolar();
            self.velocity[index] =
                (damping * self.velocity[index] + stiffness * spring + force).clamp(-0.45, 0.45);
            self.position[index] = (old_position[index] + self.velocity[index]).clamp(-1.0, 1.0);
        }
    }

    fn reset(&mut self) {
        self.position = self.initial_position;
        self.velocity = [0.0; SCANNED_POINTS];
        self.phase = 0.0;
        self.haptic_phase = 0.0;
        self.rng = XorShift::new(self.seed);
    }
}

#[inline]
fn scan_points(points: &[f32; SCANNED_POINTS], phase: f32) -> f32 {
    let position = phase * SCANNED_POINTS as f32;
    let index = position as usize % SCANNED_POINTS;
    let next = (index + 1) % SCANNED_POINTS;
    mix(points[index], points[next], position - position.floor())
}

#[derive(Debug)]
struct RegisterState {
    width: u8,
    mask: u16,
    phase: u16,
    state: u16,
    initial_state: u16,
    increment: u16,
    counter: u8,
    controls: StrangeControls,
}

impl RegisterState {
    fn new(sample_rate: f32, frequency_hz: f32, seed: u32, controls: StrangeControls) -> Self {
        let width = [8_u8, 10, 12, 16][(controls.form * 3.0).round() as usize];
        let mask = width_mask(width);
        let modulus = if width == 16 {
            65_536.0
        } else {
            (1_u32 << width) as f32
        };
        let increment = (frequency_hz * modulus / sample_rate)
            .round()
            .clamp(1.0, modulus - 1.0) as u16;
        let initial_state = ((seed as u16) & mask).max(1);
        Self {
            width,
            mask,
            phase: 0,
            state: initial_state,
            initial_state,
            increment,
            counter: 0,
            controls,
        }
    }

    #[inline]
    fn sample(&mut self) -> StrangeFrame {
        self.counter = self.counter.wrapping_add(1);
        let update_divisor = 1 + (5.0 * (1.0 - self.controls.motion)).round() as u8;
        if self.counter % update_divisor == 0 {
            let total = u32::from(self.phase) + u32::from(self.increment);
            let carry = total > u32::from(self.mask);
            let old_phase = self.phase;
            self.phase = total as u16 & self.mask;
            let borrow = self.phase < old_phase;
            let rotate = 1 + (self.controls.warp * f32::from(self.width - 2)).round() as u8;
            let rotated = rotate_width(self.state, rotate, self.width);
            let coupled = rotate_width(self.phase, self.width / 3, self.width);
            let carry_word = u16::from(carry && self.controls.chaos > 0.15) << (self.width - 1);
            let borrow_word = u16::from(borrow && self.controls.chaos > 0.55) << (self.width / 2);
            self.state = rotated
                .wrapping_add(self.phase ^ coupled)
                .wrapping_add(carry_word)
                .wrapping_sub(borrow_word)
                .wrapping_add((self.controls.couple * f32::from(self.state)) as u16)
                & self.mask;
        }
        let phase_anchor = if self.phase & (1 << (self.width - 1)) == 0 {
            -1.0
        } else {
            1.0
        };
        let word = 2.0 * f32::from(self.state) / f32::from(self.mask) - 1.0;
        let alternate_word = rotate_width(self.state ^ self.phase, self.width / 2, self.width);
        let alternate = 2.0 * f32::from(alternate_word) / f32::from(self.mask) - 1.0;
        let center = mix(phase_anchor, word, 0.25 + 0.65 * self.controls.color);
        let right = mix(center, alternate, self.controls.space);
        StrangeFrame {
            left: (0.68 * center).clamp(-1.0, 1.0),
            right: (0.68 * right).clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.phase = 0;
        self.state = self.initial_state;
        self.counter = 0;
    }
}

#[derive(Clone, Copy, Debug)]
struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(coefficient: f32) -> Self {
        Self {
            coefficient,
            state: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input - self.state);
        self.state
    }

    fn reset(&mut self) {
        self.state = 0.0;
    }

    fn set_coefficient(&mut self, coefficient: f32) {
        self.coefficient = coefficient;
    }
}

#[inline]
fn tone_coefficient(sample_rate: f32, color: f32) -> f32 {
    let cutoff_hz = 60.0 + 18_000.0 * color * color * color;
    let x = std::f32::consts::TAU * cutoff_hz / sample_rate;
    (12.0 * x / (12.0 + 6.0 * x + x * x)).clamp(0.001, 0.95)
}

#[derive(Clone, Copy, Debug)]
struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    output: f32,
}

impl DcBlocker {
    fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            coefficient: (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp(),
            previous_input: 0.0,
            output: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.output = input - self.previous_input + self.coefficient * self.output;
        self.previous_input = input;
        self.output
    }

    fn reset(&mut self) {
        self.previous_input = 0.0;
        self.output = 0.0;
    }
}

#[derive(Clone, Copy, Debug)]
struct XorShift {
    state: u32,
}

impl XorShift {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0x6d2b_79f5 } else { seed },
        }
    }

    #[inline]
    fn bipolar(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state >> 8) as f32 / 8_388_607.5 - 1.0
    }
}

#[inline]
fn sanitize(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

#[inline]
fn advance(phase: f32, increment: f32) -> f32 {
    let phase = phase + increment;
    phase - phase.floor()
}

#[inline]
fn triangle(phase: f32) -> f32 {
    1.0 - 4.0 * (phase - 0.5).abs()
}

#[inline]
fn warp_phase(phase: f32, amount: f32) -> f32 {
    let skew = 0.82 * (2.0 * amount - 1.0);
    if skew >= 0.0 {
        phase / (1.0 + skew * (1.0 - phase))
    } else {
        let mirrored = 1.0 - phase;
        1.0 - mirrored / (1.0 - skew * phase)
    }
}

#[inline]
fn mix(first: f32, second: f32, amount: f32) -> f32 {
    first + amount * (second - first)
}

#[inline]
fn movement_sign(value: f32) -> f32 {
    if value >= 0.0 { 1.0 } else { -1.0 }
}

#[inline]
fn fifth_harmonic(value: f32) -> f32 {
    let squared = value * value;
    value * (16.0 * squared * squared - 20.0 * squared + 5.0)
}

fn closed_cubic(points: &[f32; 8], phase: f32) -> f32 {
    let position = phase * points.len() as f32;
    let index = position as usize % points.len();
    let t = position - position.floor();
    let p0 = points[(index + points.len() - 1) % points.len()];
    let p1 = points[index];
    let p2 = points[(index + 1) % points.len()];
    let p3 = points[(index + 2) % points.len()];
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

const fn width_mask(width: u8) -> u16 {
    if width >= 16 {
        u16::MAX
    } else {
        ((1_u32 << width) - 1) as u16
    }
}

const fn rotate_width(value: u16, amount: u8, width: u8) -> u16 {
    let mask = width_mask(width);
    let amount = amount % width;
    if amount == 0 {
        value & mask
    } else {
        ((value << amount) | (value >> (width - amount))) & mask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_replays_every_type_exactly() {
        for kind in StrangeType::ALL {
            let mut voice =
                StrangeVoice::new(kind, 48_000.0, 220.0, 0x1234, StrangeControls::MIDPOINT)
                    .unwrap();
            let first: Vec<_> = (0..2_048).map(|_| voice.sample()).collect();
            voice.reset();
            let replay: Vec<_> = (0..2_048).map(|_| voice.sample()).collect();
            assert_eq!(first, replay, "type={kind:?}");
        }
    }

    #[test]
    fn type_selection_handles_non_finite_input() {
        assert_eq!(
            StrangeType::from_normalized(f32::NAN),
            StrangeType::Triangle
        );
    }
}
