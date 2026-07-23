use super::HybridFrame;
use super::primitives::{
    AllPass, DcBlocker, DelayResonator, ImpactEnvelope, MovingDelay, OnePoleSplit, PhaseOsc,
    RegisterOsc, soft_asymmetric, soft_clip,
};

#[derive(Debug)]
pub(super) struct CrossCoupledMachine {
    carrier: PhaseOsc,
    modulator: PhaseOsc,
    sub: PhaseOsc,
    harmonic_2: PhaseOsc,
    harmonic_3: PhaseOsc,
    harmonic_5: PhaseOsc,
    vibrato: PhaseOsc,
    pulse_motion: PhaseOsc,
    stereo_left_motion: PhaseOsc,
    stereo_right_motion: PhaseOsc,
    register: RegisterOsc,
    impact: ImpactEnvelope,
    split: OnePoleSplit,
    sub_phase: AllPass,
    left_body: DelayResonator<2048>,
    right_body: DelayResonator<2048>,
    left_delay: MovingDelay<512>,
    right_delay: MovingDelay<512>,
    left_dc: DcBlocker,
    right_dc: DcBlocker,
    previous_left_body: f32,
    previous_right_body: f32,
}

impl CrossCoupledMachine {
    pub(super) fn new(sample_rate: f32, frequency: f32, seed: u32) -> Self {
        let phase = (seed & 0xffff) as f32 / 65_536.0;
        let base_delay = sample_rate / frequency;
        let mut impact = ImpactEnvelope::new(sample_rate);
        impact.trigger();
        Self {
            carrier: PhaseOsc::new(sample_rate, frequency, phase),
            modulator: PhaseOsc::new(sample_rate, 2.01 * frequency, phase + 0.19),
            sub: PhaseOsc::new(sample_rate, 0.5 * frequency, phase + 0.37),
            harmonic_2: PhaseOsc::new(sample_rate, 2.0 * frequency, phase + 0.07),
            harmonic_3: PhaseOsc::new(sample_rate, 3.0 * frequency, phase + 0.13),
            harmonic_5: PhaseOsc::new(sample_rate, 5.0 * frequency, phase + 0.23),
            vibrato: PhaseOsc::new(sample_rate, 4.73, phase + 0.31),
            pulse_motion: PhaseOsc::new(sample_rate, 0.61, phase + 0.11),
            stereo_left_motion: PhaseOsc::new(sample_rate, 0.37, phase + 0.29),
            stereo_right_motion: PhaseOsc::new(sample_rate, 0.43, phase + 0.71),
            register: RegisterOsc::new(12, sample_rate, frequency, seed as u16 ^ 0x6d2b, 5),
            impact,
            split: OnePoleSplit::new(sample_rate, (3.5 * frequency).clamp(180.0, 1_200.0)),
            sub_phase: AllPass::new(0.47),
            left_body: DelayResonator::new(base_delay * 0.997, 0.87, 0.31),
            right_body: DelayResonator::new(base_delay * 0.754, 0.84, 0.27),
            left_delay: MovingDelay::new(73.0, 11.0),
            right_delay: MovingDelay::new(113.0, 17.0),
            left_dc: DcBlocker::new(sample_rate, 8.0),
            right_dc: DcBlocker::new(sample_rate, 8.0),
            previous_left_body: 0.0,
            previous_right_body: 0.0,
        }
    }

    #[inline]
    pub(super) fn sample(&mut self) -> HybridFrame {
        let impact = self.impact.sample();
        let machine = self.register.sample();
        let vibrato = self.vibrato.sample();
        let carrier = self.carrier.sample();
        let modulator = self.modulator.sample();
        let interaction = soft_clip(carrier + (0.26 + 0.04 * vibrato) * modulator * carrier);
        let spine = 0.11 * self.harmonic_2.sample()
            + 0.07 * self.harmonic_3.sample()
            + 0.035 * self.harmonic_5.sample();
        let onset = impact.cue * (0.16 * self.harmonic_5.sample() + 0.08 * machine.value);
        let body = impact.body * (0.58 * interaction + spine);
        let (low, high) = self.split.sample(body + onset);
        let pulse = 0.5 + 0.5 * self.pulse_motion.sample();
        let low_driven = soft_clip(low * (1.25 + 0.9 * pulse));
        let high_generated = soft_asymmetric(high, 2.1) - 0.42 * high;
        let sub = self.sub_phase.sample(self.sub.sample()) * (0.08 + 0.035 * machine.value);
        let center = 0.64 * low_driven + 0.22 * high + 0.16 * high_generated + sub;
        let event = if machine.carry || machine.borrow {
            0.045 * machine.value
        } else {
            0.0
        };
        let left_body = self
            .left_body
            .sample(0.24 * center + event, 0.055 * self.previous_right_body);
        let right_body = self
            .right_body
            .sample(0.21 * center - event, 0.047 * self.previous_left_body);
        self.previous_left_body = left_body;
        self.previous_right_body = right_body;
        let left_move = self.stereo_left_motion.sample();
        let right_move = self.stereo_right_motion.sample();
        let left_space = self
            .left_delay
            .sample(left_body + 0.12 * high_generated, left_move);
        let right_space = self
            .right_delay
            .sample(right_body - 0.10 * high_generated, right_move);
        HybridFrame {
            left: self
                .left_dc
                .sample(0.48 * center + 0.36 * left_body + 0.24 * left_space)
                .clamp(-1.0, 1.0),
            right: self
                .right_dc
                .sample(0.48 * center + 0.36 * right_body + 0.24 * right_space)
                .clamp(-1.0, 1.0),
        }
    }

    pub(super) fn reset(&mut self) {
        self.carrier.reset();
        self.modulator.reset();
        self.sub.reset();
        self.harmonic_2.reset();
        self.harmonic_3.reset();
        self.harmonic_5.reset();
        self.vibrato.reset();
        self.pulse_motion.reset();
        self.stereo_left_motion.reset();
        self.stereo_right_motion.reset();
        self.register.reset();
        self.impact.reset();
        self.impact.trigger();
        self.split.reset();
        self.sub_phase.reset();
        self.left_body.reset();
        self.right_body.reset();
        self.left_delay.reset();
        self.right_delay.reset();
        self.left_dc.reset();
        self.right_dc.reset();
        self.previous_left_body = 0.0;
        self.previous_right_body = 0.0;
    }
}
