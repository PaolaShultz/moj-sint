use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub(super) struct PhaseOsc {
    sine: f32,
    cosine: f32,
    initial_sine: f32,
    initial_cosine: f32,
    rotation_sine: f32,
    rotation_cosine: f32,
}

impl PhaseOsc {
    pub(super) fn new(sample_rate: f32, frequency_hz: f32, initial_phase: f32) -> Self {
        let angle = TAU * frequency_hz / sample_rate;
        let phase = TAU * initial_phase.fract();
        let (rotation_sine, rotation_cosine) = angle.sin_cos();
        let (initial_sine, initial_cosine) = phase.sin_cos();
        Self {
            sine: initial_sine,
            cosine: initial_cosine,
            initial_sine,
            initial_cosine,
            rotation_sine,
            rotation_cosine,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self) -> f32 {
        let output = self.sine;
        let next_sine =
            self.sine * self.rotation_cosine + self.cosine * self.rotation_sine;
        self.cosine =
            self.cosine * self.rotation_cosine - self.sine * self.rotation_sine;
        self.sine = next_sine;
        output
    }

    pub(super) fn reset(&mut self) {
        self.sine = self.initial_sine;
        self.cosine = self.initial_cosine;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RegisterSample {
    pub(super) value: f32,
    pub(super) carry: bool,
    pub(super) borrow: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RegisterOsc {
    width: u8,
    mask: u16,
    phase: u16,
    state: u16,
    initial_state: u16,
    increment: u16,
    rotate: u8,
}

impl RegisterOsc {
    pub(super) fn new(
        width: u8,
        sample_rate: f32,
        frequency_hz: f32,
        seed: u16,
        rotate: u8,
    ) -> Self {
        let width = width.clamp(2, 16);
        let mask = if width == 16 {
            u16::MAX
        } else {
            ((1_u32 << width) - 1) as u16
        };
        let modulus = f32::from(mask) + 1.0;
        let increment = (frequency_hz * modulus / sample_rate)
            .round()
            .clamp(1.0, modulus - 1.0) as u16;
        let initial_state = (seed & mask).max(1);
        Self {
            width,
            mask,
            phase: 0,
            state: initial_state,
            initial_state,
            increment,
            rotate: rotate % width,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self) -> RegisterSample {
        let previous = self.phase;
        let sum = u32::from(self.phase) + u32::from(self.increment);
        let carry = sum > u32::from(self.mask);
        self.phase = (sum as u16) & self.mask;
        let borrow = self.phase < previous;
        let amount = self.rotate;
        let rotated = if amount == 0 {
            self.state
        } else {
            ((self.state << amount) | (self.state >> (self.width - amount))) & self.mask
        };
        self.state = rotated
            .wrapping_add(self.phase ^ (self.phase >> (self.width / 2)))
            .wrapping_add(u16::from(carry) << (self.width - 1))
            .wrapping_sub(u16::from(borrow) << (self.width / 2))
            & self.mask;
        let word = self.state ^ self.phase;
        RegisterSample {
            value: 2.0 * f32::from(word & self.mask) / f32::from(self.mask) - 1.0,
            carry,
            borrow,
        }
    }

    pub(super) fn reset(&mut self) {
        self.phase = 0;
        self.state = self.initial_state;
    }

    #[cfg(test)]
    pub(super) fn width(&self) -> u8 {
        self.width
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct ImpactSample {
    pub(super) cue: f32,
    pub(super) body: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ImpactEnvelope {
    sample: usize,
    cue_samples: usize,
    body_samples: usize,
    active: bool,
}

impl ImpactEnvelope {
    pub(super) fn new(sample_rate: f32) -> Self {
        Self {
            sample: 0,
            cue_samples: (0.004 * sample_rate).round().max(1.0) as usize,
            body_samples: (0.024 * sample_rate).round().max(1.0) as usize,
            active: false,
        }
    }

    pub(super) fn trigger(&mut self) {
        self.sample = 0;
        self.active = true;
    }

    #[inline]
    pub(super) fn sample(&mut self) -> ImpactSample {
        if !self.active {
            return ImpactSample::default();
        }
        let cue_position = self.sample as f32 / self.cue_samples as f32;
        let cue = if cue_position < 1.0 {
            let triangle = 1.0 - (2.0 * cue_position - 1.0).abs();
            triangle * triangle
        } else {
            0.0
        };
        let body_position = (self.sample as f32 / self.body_samples as f32).min(1.0);
        let body = body_position * (2.0 - body_position);
        self.sample = self.sample.saturating_add(1);
        ImpactSample { cue, body }
    }

    pub(super) fn reset(&mut self) {
        self.sample = 0;
        self.active = false;
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct OnePoleSplit {
    coefficient: f32,
    low: f32,
}

impl OnePoleSplit {
    pub(super) fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            coefficient: 1.0 - (-TAU * cutoff_hz / sample_rate).exp(),
            low: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, input: f32) -> (f32, f32) {
        self.low += self.coefficient * (input - self.low);
        (self.low, input - self.low)
    }

    pub(super) fn reset(&mut self) {
        self.low = 0.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AllPass {
    coefficient: f32,
    state: f32,
}

impl AllPass {
    pub(super) fn new(coefficient: f32) -> Self {
        Self {
            coefficient: coefficient.clamp(-0.95, 0.95),
            state: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, input: f32) -> f32 {
        let output = -self.coefficient * input + self.state;
        self.state = input + self.coefficient * output;
        output
    }

    pub(super) fn reset(&mut self) {
        self.state = 0.0;
    }
}

#[derive(Debug)]
pub(super) struct MovingDelay<const N: usize> {
    buffer: [f32; N],
    write: usize,
    base: f32,
    depth: f32,
}

impl<const N: usize> MovingDelay<N> {
    pub(super) fn new(base: f32, depth: f32) -> Self {
        Self {
            buffer: [0.0; N],
            write: 0,
            base: base.clamp(2.0, N as f32 - 2.0),
            depth: depth.abs().min(base - 2.0),
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, input: f32, movement: f32) -> f32 {
        let delay = self.base + self.depth * movement.clamp(-1.0, 1.0);
        let mut position = self.write as f32 - delay;
        while position < 0.0 {
            position += N as f32;
        }
        while position >= N as f32 {
            position -= N as f32;
        }
        let first = position as usize;
        let second = if first + 1 == N { 0 } else { first + 1 };
        let fraction = position - first as f32;
        let output =
            self.buffer[first] + fraction * (self.buffer[second] - self.buffer[first]);
        self.buffer[self.write] = input;
        self.write += 1;
        if self.write == N {
            self.write = 0;
        }
        output
    }

    pub(super) fn reset(&mut self) {
        self.buffer = [0.0; N];
        self.write = 0;
    }

    #[cfg(test)]
    pub(super) fn minimum_delay(&self) -> f32 {
        self.base - self.depth
    }

    #[cfg(test)]
    pub(super) fn maximum_delay(&self) -> f32 {
        self.base + self.depth
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    pub(super) fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            coefficient: (-TAU * cutoff_hz / sample_rate).exp(),
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, input: f32) -> f32 {
        let output =
            input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }

    pub(super) fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }
}

#[derive(Debug)]
pub(super) struct DelayResonator<const N: usize> {
    buffer: [f32; N],
    write: usize,
    delay: f32,
    feedback: f32,
    damping: f32,
    lowpass: f32,
}

impl<const N: usize> DelayResonator<N> {
    pub(super) fn new(delay: f32, feedback: f32, damping: f32) -> Self {
        Self {
            buffer: [0.0; N],
            write: 0,
            delay: delay.clamp(2.0, N as f32 - 2.0),
            feedback: feedback.clamp(0.0, 0.995),
            damping: damping.clamp(0.01, 0.99),
            lowpass: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, input: f32, cross: f32) -> f32 {
        let mut position = self.write as f32 - self.delay;
        if position < 0.0 {
            position += N as f32;
        }
        let first = position as usize;
        let second = if first + 1 == N { 0 } else { first + 1 };
        let fraction = position - first as f32;
        let delayed =
            self.buffer[first] + fraction * (self.buffer[second] - self.buffer[first]);
        self.lowpass += self.damping * (delayed - self.lowpass);
        self.buffer[self.write] =
            soft_clip(input + self.feedback * self.lowpass + cross);
        self.write += 1;
        if self.write == N {
            self.write = 0;
        }
        delayed
    }

    pub(super) fn reset(&mut self) {
        self.buffer = [0.0; N];
        self.write = 0;
        self.lowpass = 0.0;
    }
}

#[inline]
pub(super) fn soft_clip(input: f32) -> f32 {
    let input = input.clamp(-3.0, 3.0);
    input / (1.0 + input.abs())
}

#[inline]
pub(super) fn soft_asymmetric(input: f32, drive: f32) -> f32 {
    let bias = 0.17;
    (soft_clip(drive * input + bias) - soft_clip(bias)).clamp(-1.0, 1.0)
}
