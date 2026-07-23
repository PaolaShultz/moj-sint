use super::HybridFrame;
use super::primitives::{
    DcBlocker, DelayResonator, ImpactEnvelope, MovingDelay, PhaseOsc, RegisterOsc,
    soft_asymmetric, soft_clip,
};

#[derive(Debug)]
pub(super) struct DualResonantBody {
    carrier: PhaseOsc,
    upper: PhaseOsc,
    left_motion: PhaseOsc,
    right_motion: PhaseOsc,
    register: RegisterOsc,
    impact: ImpactEnvelope,
    noise_state: u32,
    excitation_remaining: usize,
    initial_excitation_samples: usize,
    low_a: DelayResonator<2048>,
    low_b: DelayResonator<2048>,
    high_a: DelayResonator<2048>,
    high_b: DelayResonator<2048>,
    left_delay: MovingDelay<512>,
    right_delay: MovingDelay<512>,
    left_dc: DcBlocker,
    right_dc: DcBlocker,
    previous_low: f32,
    previous_high: f32,
}

impl DualResonantBody {
    pub(super) fn new(sample_rate: f32, frequency: f32, seed: u32) -> Self {
        let phase = (seed & 0xffff) as f32 / 65_536.0;
        let base = sample_rate / frequency;
        let excitation_samples = (0.018 * sample_rate).round() as usize;
        let mut impact = ImpactEnvelope::new(sample_rate);
        impact.trigger();
        Self {
            carrier: PhaseOsc::new(sample_rate, frequency, phase),
            upper: PhaseOsc::new(sample_rate, 2.013 * frequency, phase + 0.27),
            left_motion: PhaseOsc::new(sample_rate, 0.23, phase + 0.13),
            right_motion: PhaseOsc::new(sample_rate, 0.41, phase + 0.61),
            register: RegisterOsc::new(10, sample_rate, frequency, seed as u16 ^ 0xa53c, 3),
            impact,
            noise_state: seed ^ 0x9e37_79b9,
            excitation_remaining: excitation_samples,
            initial_excitation_samples: excitation_samples,
            low_a: DelayResonator::new(base * 0.998, 0.91, 0.24),
            low_b: DelayResonator::new(base * 0.501, 0.85, 0.31),
            high_a: DelayResonator::new(base * 0.754, 0.87, 0.37),
            high_b: DelayResonator::new(base * 0.618, 0.82, 0.43),
            left_delay: MovingDelay::new(83.0, 12.0),
            right_delay: MovingDelay::new(151.0, 19.0),
            left_dc: DcBlocker::new(sample_rate, 9.0),
            right_dc: DcBlocker::new(sample_rate, 9.0),
            previous_low: 0.0,
            previous_high: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self) -> HybridFrame {
        let impact = self.impact.sample();
        let machine = self.register.sample();
        let noise = if self.excitation_remaining > 0 {
            self.excitation_remaining -= 1;
            self.noise_state ^= self.noise_state << 13;
            self.noise_state ^= self.noise_state >> 17;
            self.noise_state ^= self.noise_state << 5;
            ((self.noise_state >> 8) as f32 / 8_388_607.5 - 1.0) * impact.cue
        } else {
            0.0
        };
        let tonal = impact.body
            * (0.12 * self.carrier.sample() + 0.045 * self.upper.sample());
        let event = if machine.carry || machine.borrow {
            0.055 * machine.value
        } else {
            0.0
        };
        let excitation = tonal + 0.18 * noise + event;
        let low_a = self
            .low_a
            .sample(excitation, 0.035 * self.previous_high);
        let low_b = self
            .low_b
            .sample(0.38 * excitation, 0.021 * self.previous_high);
        let high_a = self
            .high_a
            .sample(0.52 * excitation, 0.031 * self.previous_low);
        let high_b = self
            .high_b
            .sample(-0.31 * excitation, 0.018 * self.previous_low);
        let low_body = soft_clip(1.45 * (0.68 * low_a + 0.32 * low_b));
        let high_body = soft_asymmetric(0.61 * high_a + 0.39 * high_b, 1.65);
        self.previous_low = low_body;
        self.previous_high = high_body;
        let center = 0.42 * excitation + 0.28 * low_body;
        let left_space = self
            .left_delay
            .sample(0.58 * low_body + 0.42 * high_body, self.left_motion.sample());
        let right_space = self
            .right_delay
            .sample(0.63 * low_body - 0.37 * high_body, self.right_motion.sample());
        HybridFrame {
            left: self
                .left_dc
                .sample(center + 0.42 * left_space + 0.12 * high_body)
                .clamp(-1.0, 1.0),
            right: self
                .right_dc
                .sample(center + 0.42 * right_space - 0.10 * high_body)
                .clamp(-1.0, 1.0),
        }
    }

    pub(super) fn reset(&mut self) {
        self.carrier.reset();
        self.upper.reset();
        self.left_motion.reset();
        self.right_motion.reset();
        self.register.reset();
        self.impact.reset();
        self.impact.trigger();
        self.excitation_remaining = self.initial_excitation_samples;
        self.low_a.reset();
        self.low_b.reset();
        self.high_a.reset();
        self.high_b.reset();
        self.left_delay.reset();
        self.right_delay.reset();
        self.left_dc.reset();
        self.right_dc.reset();
        self.previous_low = 0.0;
        self.previous_high = 0.0;
    }
}
