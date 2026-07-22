use thiserror::Error;

const SINE_TABLE_SIZE: usize = 2_048;

const fn sine_approximation(mut phase: f32) -> f32 {
    while phase >= 1.0 {
        phase -= 1.0;
    }
    while phase < 0.0 {
        phase += 1.0;
    }
    let (x, sign) = if phase <= 0.5 {
        (phase * std::f32::consts::TAU, 1.0)
    } else {
        ((phase - 0.5) * std::f32::consts::TAU, -1.0)
    };
    let product = x * (std::f32::consts::PI - x);
    sign * 16.0 * product / (5.0 * std::f32::consts::PI * std::f32::consts::PI - 4.0 * product)
}

const fn build_sine_table() -> [f32; SINE_TABLE_SIZE + 1] {
    let mut table = [0.0; SINE_TABLE_SIZE + 1];
    let mut index = 0;
    while index <= SINE_TABLE_SIZE {
        table[index] = sine_approximation(index as f32 / SINE_TABLE_SIZE as f32);
        index += 1;
    }
    table
}

static SINE_TABLE: [f32; SINE_TABLE_SIZE + 1] = build_sine_table();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResearchFamily {
    NonlinearPm,
    ExcitedComb,
    SpatialMicroDelay,
    SpectralTraversal,
    IntegerSwarm,
}

impl ResearchFamily {
    pub const ALL: [Self; 5] = [
        Self::NonlinearPm,
        Self::ExcitedComb,
        Self::SpatialMicroDelay,
        Self::SpectralTraversal,
        Self::IntegerSwarm,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StereoFrame {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ResearchError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("frequency must be finite, positive, and below Nyquist")]
    InvalidFrequency,
}

#[derive(Clone, Copy, Debug)]
struct NonlinearPm {
    carrier_phase: f32,
    modulator_phase: f32,
    carrier_increment: f32,
    modulator_increment: f32,
}

impl NonlinearPm {
    fn new(sample_rate: f32, frequency_hz: f32) -> Self {
        Self {
            carrier_phase: 0.0,
            modulator_phase: 0.0,
            carrier_increment: frequency_hz / sample_rate,
            modulator_increment: 3.0 * frequency_hz / sample_rate,
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let modulator = sine_lookup(self.modulator_phase);
        let shaped = 1.45 * modulator - 0.45 * modulator * modulator * modulator;
        let output = 0.72 * sine_lookup(self.carrier_phase + 0.28 * shaped);
        self.carrier_phase = advance_phase(self.carrier_phase, self.carrier_increment);
        self.modulator_phase = advance_phase(self.modulator_phase, self.modulator_increment);
        output.clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        self.carrier_phase = 0.0;
        self.modulator_phase = 0.0;
    }
}

const COMB_BUFFER_SIZE: usize = 1_024;

#[derive(Debug)]
struct CombLine {
    buffer: [f32; COMB_BUFFER_SIZE],
    write_index: usize,
    delay_samples: f32,
    lowpass: f32,
    feedback: f32,
    damping: f32,
}

impl CombLine {
    fn new(delay_samples: f32, feedback: f32, damping: f32) -> Self {
        Self {
            buffer: [0.0; COMB_BUFFER_SIZE],
            write_index: 0,
            delay_samples,
            lowpass: 0.0,
            feedback,
            damping,
        }
    }

    #[inline]
    fn sample(&mut self, excitation: f32) -> f32 {
        let mut read_position = self.write_index as f32 - self.delay_samples;
        if read_position < 0.0 {
            read_position += COMB_BUFFER_SIZE as f32;
        }
        let first = read_position as usize;
        let second = if first + 1 == COMB_BUFFER_SIZE {
            0
        } else {
            first + 1
        };
        let fraction = read_position - first as f32;
        let delayed = self.buffer[first] + fraction * (self.buffer[second] - self.buffer[first]);
        self.lowpass += self.damping * (delayed - self.lowpass);
        self.buffer[self.write_index] =
            (excitation + self.feedback * self.lowpass).clamp(-1.0, 1.0);
        self.write_index += 1;
        if self.write_index == COMB_BUFFER_SIZE {
            self.write_index = 0;
        }
        delayed
    }

    fn reset(&mut self) {
        self.buffer = [0.0; COMB_BUFFER_SIZE];
        self.write_index = 0;
        self.lowpass = 0.0;
    }
}

#[derive(Debug)]
struct ExcitedComb {
    lines: [CombLine; 3],
    excitation_remaining: usize,
    initial_excitation_samples: usize,
    noise_state: u32,
    previous_output: f32,
    dc_output: f32,
    dc_coefficient: f32,
}

impl ExcitedComb {
    fn new(sample_rate: f32, frequency_hz: f32) -> Self {
        let base_delay = (sample_rate / frequency_hz).clamp(2.0, COMB_BUFFER_SIZE as f32 - 2.0);
        let excitation_samples = base_delay.ceil() as usize;
        Self {
            lines: [
                CombLine::new(base_delay, 0.998, 0.42),
                CombLine::new((base_delay * 0.754_877_7).max(2.0), 0.996, 0.34),
                CombLine::new((base_delay * 0.612_913_1).max(2.0), 0.994, 0.28),
            ],
            excitation_remaining: excitation_samples,
            initial_excitation_samples: excitation_samples,
            noise_state: 0x6d2b_79f5,
            previous_output: 0.0,
            dc_output: 0.0,
            dc_coefficient: (-std::f32::consts::TAU * 10.0 / sample_rate).exp(),
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let excitation = if self.excitation_remaining > 0 {
            self.excitation_remaining -= 1;
            self.noise_state ^= self.noise_state << 13;
            self.noise_state ^= self.noise_state >> 17;
            self.noise_state ^= self.noise_state << 5;
            ((self.noise_state >> 8) as f32 / 8_388_607.5 - 1.0) * 0.42
        } else {
            0.0
        };
        let output = 0.56 * self.lines[0].sample(excitation)
            + 0.26 * self.lines[1].sample(0.72 * excitation)
            + 0.18 * self.lines[2].sample(-0.55 * excitation);
        self.dc_output = output - self.previous_output + self.dc_coefficient * self.dc_output;
        self.previous_output = output;
        self.dc_output.clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
        self.excitation_remaining = self.initial_excitation_samples;
        self.noise_state = 0x6d2b_79f5;
        self.previous_output = 0.0;
        self.dc_output = 0.0;
    }

    fn delay_bounds(&self) -> (f32, f32) {
        self.lines.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(minimum, maximum), line| {
                (
                    minimum.min(line.delay_samples),
                    maximum.max(line.delay_samples),
                )
            },
        )
    }
}

const MICRO_DELAY_BUFFER_SIZE: usize = 512;

#[derive(Debug)]
struct MicroDelayLine {
    buffer: [f32; MICRO_DELAY_BUFFER_SIZE],
    write_index: usize,
    base_delay: f32,
    depth: f32,
}

impl MicroDelayLine {
    fn new(base_delay: f32, depth: f32) -> Self {
        Self {
            buffer: [0.0; MICRO_DELAY_BUFFER_SIZE],
            write_index: 0,
            base_delay,
            depth,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32, movement: f32) -> f32 {
        let delay = self.base_delay + self.depth * movement.clamp(-1.0, 1.0);
        let mut read_position = self.write_index as f32 - delay;
        if read_position < 0.0 {
            read_position += MICRO_DELAY_BUFFER_SIZE as f32;
        }
        let first = read_position as usize;
        let second = if first + 1 == MICRO_DELAY_BUFFER_SIZE {
            0
        } else {
            first + 1
        };
        let fraction = read_position - first as f32;
        let output = self.buffer[first] + fraction * (self.buffer[second] - self.buffer[first]);
        self.buffer[self.write_index] = input;
        self.write_index += 1;
        if self.write_index == MICRO_DELAY_BUFFER_SIZE {
            self.write_index = 0;
        }
        output
    }

    fn reset(&mut self) {
        self.buffer = [0.0; MICRO_DELAY_BUFFER_SIZE];
        self.write_index = 0;
    }
}

#[derive(Debug)]
struct SpatialMicroDelay {
    source_phase: f32,
    source_increment: f32,
    active_harmonics: usize,
    movement_phases: [f32; 4],
    movement_increments: [f32; 4],
    lines: [MicroDelayLine; 4],
}

impl SpatialMicroDelay {
    fn new(sample_rate: f32, frequency_hz: f32) -> Self {
        let active_harmonics = ((0.45 * sample_rate / frequency_hz) as usize).clamp(1, 7);
        Self {
            source_phase: 0.0,
            source_increment: frequency_hz / sample_rate,
            active_harmonics,
            movement_phases: [0.0, 0.25, 0.5, 0.75],
            movement_increments: [
                0.17 / sample_rate,
                0.23 / sample_rate,
                0.13 / sample_rate,
                0.19 / sample_rate,
            ],
            lines: [
                MicroDelayLine::new(36.0, 4.0),
                MicroDelayLine::new(77.0, 5.0),
                MicroDelayLine::new(139.0, 7.0),
                MicroDelayLine::new(245.0, 8.0),
            ],
        }
    }

    #[inline]
    fn sample(&mut self) -> StereoFrame {
        let mut buzz = 0.0_f32;
        let mut weight_sum = 0.0_f32;
        for harmonic in 1..=self.active_harmonics {
            let weight = 1.0 / harmonic as f32;
            buzz += weight * sine_lookup(self.source_phase * harmonic as f32);
            weight_sum += weight;
        }
        buzz *= 0.62 / weight_sum.max(1.0);
        self.source_phase = advance_phase(self.source_phase, self.source_increment);

        let mut taps = [0.0_f32; 4];
        for (index, tap) in taps.iter_mut().enumerate() {
            let triangle = 1.0 - 4.0 * (self.movement_phases[index] - 0.5).abs();
            *tap = self.lines[index].sample(buzz, triangle);
            self.movement_phases[index] =
                advance_phase(self.movement_phases[index], self.movement_increments[index]);
        }
        let left = 0.55 * buzz + 0.28 * taps[0] + 0.15 * taps[2] - 0.03 * taps[3];
        let right = 0.55 * buzz + 0.28 * taps[1] + 0.15 * taps[3] - 0.03 * taps[2];
        StereoFrame {
            left: left.clamp(-1.0, 1.0),
            right: right.clamp(-1.0, 1.0),
        }
    }

    fn reset(&mut self) {
        self.source_phase = 0.0;
        self.movement_phases = [0.0, 0.25, 0.5, 0.75];
        for line in &mut self.lines {
            line.reset();
        }
    }

    fn delay_bounds(&self) -> (f32, f32) {
        self.lines.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(minimum, maximum), line| {
                (
                    minimum.min(line.base_delay - line.depth),
                    maximum.max(line.base_delay + line.depth),
                )
            },
        )
    }
}

const SPECTRAL_PARTIALS: usize = 16;
const SPECTRAL_FRAMES: [[f32; SPECTRAL_PARTIALS]; 4] = [
    [
        1.0, 0.04, 0.78, 0.03, 0.52, 0.02, 0.34, 0.01, 0.22, 0.0, 0.14, 0.0, 0.08, 0.0, 0.04, 0.0,
    ],
    [
        1.0, 0.55, 0.12, 0.72, 0.18, 0.64, 0.08, 0.38, 0.05, 0.22, 0.03, 0.12, 0.02, 0.07, 0.01,
        0.03,
    ],
    [
        1.0, 0.02, 0.06, 0.03, 0.88, 0.02, 0.08, 0.01, 0.58, 0.0, 0.06, 0.0, 0.31, 0.0, 0.04, 0.0,
    ],
    [
        1.0, 0.82, 0.68, 0.54, 0.43, 0.34, 0.27, 0.21, 0.16, 0.12, 0.09, 0.07, 0.05, 0.04, 0.03,
        0.02,
    ],
];

#[derive(Debug)]
struct SpectralTraversal {
    sine: [f32; SPECTRAL_PARTIALS],
    cosine: [f32; SPECTRAL_PARTIALS],
    rotation_sine: [f32; SPECTRAL_PARTIALS],
    rotation_cosine: [f32; SPECTRAL_PARTIALS],
    active_partials: usize,
    movement_phase: f32,
    movement_increment: f32,
    samples_since_normalize: u16,
}

impl SpectralTraversal {
    fn new(sample_rate: f32, frequency_hz: f32) -> Self {
        let active_partials =
            ((0.45 * sample_rate / frequency_hz) as usize).clamp(1, SPECTRAL_PARTIALS);
        let mut rotation_sine = [0.0; SPECTRAL_PARTIALS];
        let mut rotation_cosine = [1.0; SPECTRAL_PARTIALS];
        for index in 0..active_partials {
            let phase_increment = frequency_hz * (index + 1) as f32 / sample_rate;
            let angle = std::f32::consts::TAU * phase_increment;
            (rotation_sine[index], rotation_cosine[index]) = angle.sin_cos();
        }
        Self {
            sine: [0.0; SPECTRAL_PARTIALS],
            cosine: [1.0; SPECTRAL_PARTIALS],
            rotation_sine,
            rotation_cosine,
            active_partials,
            movement_phase: 0.0,
            movement_increment: 0.5 / sample_rate,
            samples_since_normalize: 0,
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let traversal = if self.movement_phase <= 0.5 {
            self.movement_phase * 6.0
        } else {
            (1.0 - self.movement_phase) * 6.0
        };
        let first_frame = (traversal as usize).min(2);
        let fraction = (traversal - first_frame as f32).clamp(0.0, 1.0);
        let mut output = 0.0_f32;
        let mut amplitude_sum = 0.0_f32;
        for (index, (((sine, cosine), rotation_sine), rotation_cosine)) in self
            .sine
            .iter_mut()
            .zip(self.cosine.iter_mut())
            .zip(self.rotation_sine.iter())
            .zip(self.rotation_cosine.iter())
            .enumerate()
            .take(self.active_partials)
        {
            let amplitude = SPECTRAL_FRAMES[first_frame][index]
                + fraction
                    * (SPECTRAL_FRAMES[first_frame + 1][index]
                        - SPECTRAL_FRAMES[first_frame][index]);
            output += amplitude * *sine;
            amplitude_sum += amplitude;
            let next_sine = *sine * *rotation_cosine + *cosine * *rotation_sine;
            *cosine = *cosine * *rotation_cosine - *sine * *rotation_sine;
            *sine = next_sine;
        }
        self.samples_since_normalize += 1;
        if self.samples_since_normalize == 1_024 {
            for (sine, cosine) in self
                .sine
                .iter_mut()
                .zip(self.cosine.iter_mut())
                .take(self.active_partials)
            {
                let scale = (*sine * *sine + *cosine * *cosine).sqrt().recip();
                *sine *= scale;
                *cosine *= scale;
            }
            self.samples_since_normalize = 0;
        }
        self.movement_phase = advance_phase(self.movement_phase, self.movement_increment);
        (1.5 * output / amplitude_sum.max(1.0)).clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        self.sine = [0.0; SPECTRAL_PARTIALS];
        self.cosine = [1.0; SPECTRAL_PARTIALS];
        self.movement_phase = 0.0;
        self.samples_since_normalize = 0;
    }
}

const INTEGER_MACHINE_COUNT: usize = 24;

const fn width_mask(width: u8) -> u16 {
    if width >= 16 {
        u16::MAX
    } else {
        ((1_u32 << width) - 1) as u16
    }
}

const fn rotate_within_width(value: u16, amount: u8, width: u8) -> u16 {
    let mask = width_mask(width);
    let amount = amount % width;
    if amount == 0 {
        value & mask
    } else {
        ((value << amount) | (value >> (width - amount))) & mask
    }
}

pub const fn phase_period(width: u8, increment: u16) -> u32 {
    if increment == 0 {
        return 0;
    }
    let modulus = if width >= 16 { 65_536 } else { 1_u32 << width };
    let mut left = modulus;
    let mut right = increment as u32 % modulus;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    modulus / left
}

#[derive(Clone, Copy, Debug)]
struct SmallRegisterMachine {
    width: u8,
    phase: u16,
    state: u16,
    increment: u16,
    initial_state: u16,
    rotate: u8,
    tap: u8,
}

impl SmallRegisterMachine {
    fn new(width: u8, increment: u16, seed: u16, tap: u8) -> Self {
        let mask = width_mask(width);
        let initial_state = (seed & mask).max(1);
        Self {
            width,
            phase: 0,
            state: initial_state,
            increment: increment & mask,
            initial_state,
            rotate: (tap % (width - 1)) + 1,
            tap: tap % width,
        }
    }

    #[inline]
    fn sample_word(&mut self) -> u16 {
        let mask = width_mask(self.width);
        let sum = u32::from(self.phase) + u32::from(self.increment);
        let carry = sum > u32::from(mask);
        let previous_phase = self.phase;
        self.phase = (sum as u16) & mask;
        let borrow = self.phase < previous_phase;
        let rotated = rotate_within_width(self.state, self.rotate, self.width);
        let shifted = self.phase >> self.tap;
        let carry_word = u16::from(carry) << (self.width - 1);
        let borrow_word = u16::from(borrow) << (self.width / 2);
        self.state = rotated
            .wrapping_add(self.phase ^ shifted)
            .wrapping_add(carry_word)
            .wrapping_sub(borrow_word)
            & mask;
        (self.state ^ rotate_within_width(self.phase, self.tap, self.width)) & mask
    }

    fn reset(&mut self) {
        self.phase = 0;
        self.state = self.initial_state;
    }
}

#[derive(Debug)]
struct IntegerSwarm {
    machines: [SmallRegisterMachine; INTEGER_MACHINE_COUNT],
    previous_input: f32,
    dc_output: f32,
    dc_coefficient: f32,
}

impl IntegerSwarm {
    fn new(sample_rate: f32, frequency_hz: f32) -> Self {
        let widths = [16_u8, 12, 10, 8];
        let ratios = [1_u16, 2, 3, 5, 1, 7];
        let machines = std::array::from_fn(|index| {
            let width = widths[index % widths.len()];
            let ratio = ratios[index / widths.len()];
            let modulus = if width == 16 {
                65_536.0
            } else {
                (1_u32 << width) as f32
            };
            let increment = (frequency_hz * f32::from(ratio) * modulus / sample_rate)
                .round()
                .clamp(1.0, modulus - 1.0) as u16;
            let seed = (0x9e37_u16.wrapping_mul(index as u16 + 1)) ^ 0x5a5a;
            SmallRegisterMachine::new(width, increment, seed, (index % 7 + 1) as u8)
        });
        Self {
            machines,
            previous_input: 0.0,
            dc_output: 0.0,
            dc_coefficient: (-std::f32::consts::TAU * 8.0 / sample_rate).exp(),
        }
    }

    #[inline]
    fn sample(&mut self) -> f32 {
        let mut output = 0.0_f32;
        let mut weight_sum = 0.0_f32;
        for machine in &mut self.machines {
            let word = machine.sample_word();
            let mask = width_mask(machine.width);
            let state_component = 2.0 * f32::from(word) / f32::from(mask) - 1.0;
            let phase_component = if machine.phase & (1 << (machine.width - 1)) == 0 {
                -1.0
            } else {
                1.0
            };
            let normalized = 0.62 * phase_component + 0.38 * state_component;
            let weight = match machine.width {
                16 => 1.0,
                12 => 0.72,
                10 => 0.52,
                _ => 0.34,
            };
            output += weight * normalized;
            weight_sum += weight;
        }
        let input = 0.72 * output / weight_sum;
        self.dc_output = input - self.previous_input + self.dc_coefficient * self.dc_output;
        self.previous_input = input;
        self.dc_output.clamp(-1.0, 1.0)
    }

    fn reset(&mut self) {
        for machine in &mut self.machines {
            machine.reset();
        }
        self.previous_input = 0.0;
        self.dc_output = 0.0;
    }
}

// Keeping each disposable source's bounded state inline makes construction the
// only setup boundary and lets sample-path allocation tests cover the whole
// state transition without heap indirection.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
enum ResearchState {
    NonlinearPm(NonlinearPm),
    ExcitedComb(ExcitedComb),
    SpatialMicroDelay(SpatialMicroDelay),
    SpectralTraversal(SpectralTraversal),
    IntegerSwarm(IntegerSwarm),
}

#[derive(Debug)]
pub struct ResearchSource {
    state: ResearchState,
}

impl ResearchSource {
    pub fn new(
        family: ResearchFamily,
        sample_rate: f32,
        frequency_hz: f32,
    ) -> Result<Self, ResearchError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(ResearchError::InvalidSampleRate);
        }
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 || frequency_hz >= 0.5 * sample_rate {
            return Err(ResearchError::InvalidFrequency);
        }
        let state = match family {
            ResearchFamily::NonlinearPm => {
                ResearchState::NonlinearPm(NonlinearPm::new(sample_rate, frequency_hz))
            }
            ResearchFamily::ExcitedComb => {
                ResearchState::ExcitedComb(ExcitedComb::new(sample_rate, frequency_hz))
            }
            ResearchFamily::SpatialMicroDelay => {
                ResearchState::SpatialMicroDelay(SpatialMicroDelay::new(sample_rate, frequency_hz))
            }
            ResearchFamily::SpectralTraversal => {
                ResearchState::SpectralTraversal(SpectralTraversal::new(sample_rate, frequency_hz))
            }
            ResearchFamily::IntegerSwarm => {
                ResearchState::IntegerSwarm(IntegerSwarm::new(sample_rate, frequency_hz))
            }
        };
        Ok(Self { state })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        match &mut self.state {
            ResearchState::NonlinearPm(source) => {
                let sample = source.sample();
                StereoFrame {
                    left: sample,
                    right: sample,
                }
            }
            ResearchState::ExcitedComb(source) => {
                let sample = source.sample();
                StereoFrame {
                    left: sample,
                    right: sample,
                }
            }
            ResearchState::SpatialMicroDelay(source) => source.sample(),
            ResearchState::SpectralTraversal(source) => {
                let sample = source.sample();
                StereoFrame {
                    left: sample,
                    right: sample,
                }
            }
            ResearchState::IntegerSwarm(source) => {
                let sample = source.sample();
                StereoFrame {
                    left: sample,
                    right: sample,
                }
            }
        }
    }

    pub fn reset(&mut self) {
        match &mut self.state {
            ResearchState::NonlinearPm(source) => source.reset(),
            ResearchState::ExcitedComb(source) => source.reset(),
            ResearchState::SpatialMicroDelay(source) => source.reset(),
            ResearchState::SpectralTraversal(source) => source.reset(),
            ResearchState::IntegerSwarm(source) => source.reset(),
        }
    }

    pub fn minimum_delay_samples(&self) -> f32 {
        match &self.state {
            ResearchState::ExcitedComb(source) => source.delay_bounds().0,
            ResearchState::SpatialMicroDelay(source) => source.delay_bounds().0,
            ResearchState::NonlinearPm(_) => 0.0,
            ResearchState::SpectralTraversal(_) => 0.0,
            ResearchState::IntegerSwarm(_) => 0.0,
        }
    }

    pub fn maximum_delay_samples(&self) -> f32 {
        match &self.state {
            ResearchState::ExcitedComb(source) => source.delay_bounds().1,
            ResearchState::SpatialMicroDelay(source) => source.delay_bounds().1,
            ResearchState::NonlinearPm(_) => 0.0,
            ResearchState::SpectralTraversal(_) => 0.0,
            ResearchState::IntegerSwarm(_) => 0.0,
        }
    }

    pub fn active_partial_count(&self) -> usize {
        match &self.state {
            ResearchState::SpectralTraversal(source) => source.active_partials,
            _ => 0,
        }
    }

    pub fn active_machine_count(&self) -> usize {
        match &self.state {
            ResearchState::IntegerSwarm(_) => INTEGER_MACHINE_COUNT,
            _ => 0,
        }
    }

    pub fn register_widths(&self) -> [u8; 4] {
        match &self.state {
            ResearchState::IntegerSwarm(_) => [8, 10, 12, 16],
            _ => [0; 4],
        }
    }
}

#[inline]
fn advance_phase(mut phase: f32, increment: f32) -> f32 {
    phase += increment;
    while phase >= 1.0 {
        phase -= 1.0;
    }
    phase
}

#[inline]
fn sine_lookup(mut phase: f32) -> f32 {
    while phase < 0.0 {
        phase += 1.0;
    }
    while phase >= 1.0 {
        phase -= 1.0;
    }
    let position = phase * SINE_TABLE_SIZE as f32;
    let index = position as usize;
    let fraction = position - index as f32;
    SINE_TABLE[index] + fraction * (SINE_TABLE[index + 1] - SINE_TABLE[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    #[test]
    fn nonlinear_pm_rejects_invalid_configuration() {
        assert!(ResearchSource::new(ResearchFamily::NonlinearPm, 0.0, 440.0).is_err());
        assert!(ResearchSource::new(ResearchFamily::NonlinearPm, f32::NAN, 440.0).is_err());
        assert!(ResearchSource::new(ResearchFamily::NonlinearPm, 48_000.0, 0.0).is_err());
        assert!(ResearchSource::new(ResearchFamily::NonlinearPm, 48_000.0, 24_000.0).is_err());
    }

    #[test]
    fn nonlinear_pm_is_deterministic_bounded_resettable_and_allocation_free() {
        let mut first = ResearchSource::new(ResearchFamily::NonlinearPm, 48_000.0, 440.0).unwrap();
        let mut second = ResearchSource::new(ResearchFamily::NonlinearPm, 48_000.0, 440.0).unwrap();
        let mut energy = 0.0_f64;
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..16_384 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(a.left.is_finite() && a.right.is_finite());
                assert!(a.left.abs() <= 1.0 && a.right.abs() <= 1.0, "{a:?}");
                energy += f64::from(a.left) * f64::from(a.left);
            }
        });
        assert!(energy > 1.0);
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
    }

    #[test]
    fn nonlinear_pm_retains_note_and_generates_sidebands() {
        let sample_rate = 48_000.0_f32;
        let frequency = 500.0_f32;
        let mut source =
            ResearchSource::new(ResearchFamily::NonlinearPm, sample_rate, frequency).unwrap();
        let sample_count = 48_000;
        let mut fundamental_sine = 0.0_f64;
        let mut fundamental_cosine = 0.0_f64;
        let mut sideband_sine = 0.0_f64;
        let mut sideband_cosine = 0.0_f64;
        for index in 0..sample_count {
            let sample = f64::from(source.sample().left);
            let phase = f64::from(TAU * frequency * index as f32 / sample_rate);
            fundamental_sine += sample * phase.sin();
            fundamental_cosine += sample * phase.cos();
            let side_phase = 4.0 * phase;
            sideband_sine += sample * side_phase.sin();
            sideband_cosine += sample * side_phase.cos();
        }
        let fundamental = 2.0 * fundamental_sine.hypot(fundamental_cosine) / sample_count as f64;
        assert!(fundamental > 0.05, "fundamental={fundamental}");
        let sideband = 2.0 * sideband_sine.hypot(sideband_cosine) / sample_count as f64;
        assert!(sideband > 0.03, "sideband={sideband}");
    }

    #[test]
    fn excited_comb_is_seeded_bounded_decaying_and_allocation_free() {
        let mut first = ResearchSource::new(ResearchFamily::ExcitedComb, 48_000.0, 110.0).unwrap();
        let mut second = ResearchSource::new(ResearchFamily::ExcitedComb, 48_000.0, 110.0).unwrap();
        let mut early_energy = 0.0_f64;
        let mut late_energy = 0.0_f64;
        assert_no_alloc::assert_no_alloc(|| {
            for index in 0..48_000 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(
                    a.left.is_finite() && a.left.abs() <= 1.0,
                    "index={index} {a:?}"
                );
                let energy = f64::from(a.left) * f64::from(a.left);
                if (1_000..8_000).contains(&index) {
                    early_energy += energy;
                } else if index >= 40_000 {
                    late_energy += energy;
                }
            }
        });
        assert!(early_energy > 1.0, "early={early_energy}");
        assert!(
            late_energy > 1.0e-6 && late_energy < early_energy,
            "early={early_energy} late={late_energy}"
        );
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
    }

    #[test]
    fn excited_comb_retains_requested_pitch_and_delay_bounds() {
        let sample_rate = 48_000.0_f32;
        let frequency = 125.0_f32;
        let source =
            ResearchSource::new(ResearchFamily::ExcitedComb, sample_rate, frequency).unwrap();
        assert!(source.maximum_delay_samples() <= 1_024.0);
        assert!(source.minimum_delay_samples() >= 2.0);
        let mut source = source;
        let mut sine_sum = 0.0_f64;
        let mut cosine_sum = 0.0_f64;
        let sample_count = 48_000;
        for index in 0..sample_count {
            let sample = f64::from(source.sample().left);
            let phase = f64::from(TAU * frequency * index as f32 / sample_rate);
            sine_sum += sample * phase.sin();
            cosine_sum += sample * phase.cos();
        }
        let fundamental = 2.0 * sine_sum.hypot(cosine_sum) / sample_count as f64;
        assert!(fundamental > 0.002, "fundamental={fundamental}");
    }

    #[test]
    fn spatial_micro_delay_is_continuous_bounded_and_spatially_distinct() {
        let mut first =
            ResearchSource::new(ResearchFamily::SpatialMicroDelay, 48_000.0, 220.0).unwrap();
        let mut second =
            ResearchSource::new(ResearchFamily::SpatialMicroDelay, 48_000.0, 220.0).unwrap();
        assert!(first.minimum_delay_samples() >= 24.0);
        assert!(first.maximum_delay_samples() <= 384.0);
        let mut left_energy = 0.0_f64;
        let mut right_energy = 0.0_f64;
        let mut cross = 0.0_f64;
        let mut mid_energy = 0.0_f64;
        let mut side_energy = 0.0_f64;
        let mut mono_energy = 0.0_f64;
        let mut previous = StereoFrame::default();
        let mut maximum_jump = 0.0_f32;
        assert_no_alloc::assert_no_alloc(|| {
            for index in 0..48_000 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(a.left.is_finite() && a.right.is_finite());
                assert!(
                    a.left.abs() <= 1.0 && a.right.abs() <= 1.0,
                    "index={index} {a:?}"
                );
                maximum_jump = maximum_jump
                    .max((a.left - previous.left).abs())
                    .max((a.right - previous.right).abs());
                previous = a;
                let left = f64::from(a.left);
                let right = f64::from(a.right);
                let mid = 0.5 * (left + right);
                let side = 0.5 * (left - right);
                left_energy += left * left;
                right_energy += right * right;
                cross += left * right;
                mid_energy += mid * mid;
                side_energy += side * side;
                mono_energy += mid * mid;
            }
        });
        let correlation = cross / (left_energy * right_energy).sqrt();
        let side_to_mid = side_energy / mid_energy;
        assert!(
            (0.15..0.98).contains(&correlation),
            "correlation={correlation}"
        );
        assert!(
            (0.01..0.8).contains(&side_to_mid),
            "side_to_mid={side_to_mid}"
        );
        assert!(mono_energy > 10.0);
        assert!(maximum_jump < 0.5, "maximum_jump={maximum_jump}");
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
    }

    #[test]
    fn spectral_traversal_is_pitched_bounded_dynamic_and_allocation_free() {
        let sample_rate = 48_000.0_f32;
        let frequency = 250.0_f32;
        let mut first =
            ResearchSource::new(ResearchFamily::SpectralTraversal, sample_rate, frequency).unwrap();
        let mut second =
            ResearchSource::new(ResearchFamily::SpectralTraversal, sample_rate, frequency).unwrap();
        let mut windows = [[0.0_f64; 2]; 3];
        let starts = [2_000_usize, 22_000, 42_000];
        assert_no_alloc::assert_no_alloc(|| {
            for index in 0..48_000 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(
                    a.left.is_finite() && a.left.abs() <= 1.0,
                    "index={index} {a:?}"
                );
                for (window, start) in starts.into_iter().enumerate() {
                    if (start..start + 4_000).contains(&index) {
                        let local = index - start;
                        let third_phase =
                            TAU as f64 * 3.0 * frequency as f64 * local as f64 / sample_rate as f64;
                        windows[window][0] += f64::from(a.left) * third_phase.sin();
                        windows[window][1] += f64::from(a.left) * third_phase.cos();
                    }
                }
            }
        });
        let levels = windows.map(|pair| pair[0].hypot(pair[1]) * 2.0 / 4_000.0);
        assert!((levels[0] - levels[1]).abs() > 0.03, "levels={levels:?}");
        assert!((levels[1] - levels[2]).abs() > 0.03, "levels={levels:?}");
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
    }

    #[test]
    fn spectral_traversal_tapers_partials_above_guard_band() {
        let source =
            ResearchSource::new(ResearchFamily::SpectralTraversal, 48_000.0, 4_000.0).unwrap();
        assert_eq!(source.active_partial_count(), 5);
    }

    #[test]
    fn integer_swarm_width_arithmetic_is_explicit_and_cycles() {
        assert_eq!(width_mask(8), 0x00ff);
        assert_eq!(width_mask(10), 0x03ff);
        assert_eq!(width_mask(12), 0x0fff);
        assert_eq!(width_mask(16), 0xffff);
        assert_eq!(rotate_within_width(0x0081, 1, 8), 0x0003);
        assert_eq!(rotate_within_width(0x0201, 3, 10), 0x000c);
        assert_eq!(phase_period(8, 1), 256);
        assert_eq!(phase_period(8, 4), 64);
        assert_eq!(phase_period(12, 0), 0);
    }

    #[test]
    fn integer_swarm_is_deterministic_pitched_bounded_and_allocation_free() {
        let mut first = ResearchSource::new(ResearchFamily::IntegerSwarm, 48_000.0, 220.0).unwrap();
        let mut second =
            ResearchSource::new(ResearchFamily::IntegerSwarm, 48_000.0, 220.0).unwrap();
        assert_eq!(first.active_machine_count(), 24);
        assert_eq!(first.register_widths(), [8, 10, 12, 16]);
        let mut changes = 0_usize;
        let mut previous = 0.0_f32;
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        assert_no_alloc::assert_no_alloc(|| {
            for _ in 0..48_000 {
                let a = first.sample();
                let b = second.sample();
                assert_eq!(a, b);
                assert!(a.left.is_finite() && a.left.abs() <= 1.0, "{a:?}");
                changes += usize::from(a.left != previous);
                previous = a.left;
                hash ^= u64::from(a.left.to_bits());
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        });
        assert!(changes > 1_000, "changes={changes}");
        assert_ne!(hash, 0xcbf2_9ce4_8422_2325);
        first.reset();
        second.reset();
        assert_eq!(first.sample(), second.sample());
    }

    #[test]
    fn integer_machine_widths_have_distinct_deterministic_sequences() {
        let mut narrow = SmallRegisterMachine::new(8, 3, 0x5b, 1);
        let mut wide = SmallRegisterMachine::new(16, 3, 0x5b, 1);
        let narrow_samples: Vec<_> = (0..512).map(|_| narrow.sample_word()).collect();
        let wide_samples: Vec<_> = (0..512).map(|_| wide.sample_word()).collect();
        assert_ne!(narrow_samples, wide_samples);
        assert!(narrow_samples.windows(2).any(|pair| pair[0] != pair[1]));
        assert!(wide_samples.windows(2).any(|pair| pair[0] != pair[1]));
    }
}
