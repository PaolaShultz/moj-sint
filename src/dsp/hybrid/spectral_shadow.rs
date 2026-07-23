use super::HybridFrame;
use super::primitives::{
    AllPass, DcBlocker, ImpactEnvelope, MovingDelay, OnePoleSplit, PhaseOsc,
    RegisterOsc, soft_asymmetric,
};

const PARTIALS: usize = 10;
const FRAMES: [[f32; PARTIALS]; 4] = [
    [1.0, 0.08, 0.62, 0.03, 0.38, 0.02, 0.21, 0.01, 0.11, 0.0],
    [1.0, 0.54, 0.18, 0.67, 0.12, 0.43, 0.07, 0.24, 0.04, 0.12],
    [1.0, 0.04, 0.09, 0.03, 0.76, 0.02, 0.11, 0.01, 0.42, 0.0],
    [1.0, 0.72, 0.55, 0.39, 0.29, 0.21, 0.15, 0.10, 0.07, 0.04],
];

#[derive(Debug)]
pub(super) struct SpectralShadow {
    partials: [PhaseOsc; PARTIALS],
    active_partials: usize,
    shadow: PhaseOsc,
    traversal: PhaseOsc,
    left_motion: PhaseOsc,
    right_motion: PhaseOsc,
    address: RegisterOsc,
    address_smooth: f32,
    impact: ImpactEnvelope,
    left_split: OnePoleSplit,
    right_split: OnePoleSplit,
    left_allpass: AllPass,
    right_allpass: AllPass,
    left_delay: MovingDelay<512>,
    right_delay: MovingDelay<512>,
    left_dc: DcBlocker,
    right_dc: DcBlocker,
}

impl SpectralShadow {
    pub(super) fn new(sample_rate: f32, frequency: f32, seed: u32) -> Self {
        let phase = (seed & 0xffff) as f32 / 65_536.0;
        let active_partials = ((0.43 * sample_rate / frequency) as usize).clamp(1, PARTIALS);
        let partials = std::array::from_fn(|index| {
            PhaseOsc::new(
                sample_rate,
                frequency * (index + 1) as f32,
                phase + 0.071 * index as f32,
            )
        });
        let mut impact = ImpactEnvelope::new(sample_rate);
        impact.trigger();
        Self {
            partials,
            active_partials,
            shadow: PhaseOsc::new(sample_rate, 0.5 * frequency, phase + 0.41),
            traversal: PhaseOsc::new(sample_rate, 0.19, phase + 0.17),
            left_motion: PhaseOsc::new(sample_rate, 0.31, phase + 0.23),
            right_motion: PhaseOsc::new(sample_rate, 0.47, phase + 0.73),
            address: RegisterOsc::new(16, sample_rate, 0.73 * frequency, seed as u16, 7),
            address_smooth: 0.0,
            impact,
            left_split: OnePoleSplit::new(sample_rate, (4.2 * frequency).clamp(260.0, 1_800.0)),
            right_split: OnePoleSplit::new(sample_rate, (5.6 * frequency).clamp(310.0, 2_300.0)),
            left_allpass: AllPass::new(0.39),
            right_allpass: AllPass::new(-0.31),
            left_delay: MovingDelay::new(59.0, 9.0),
            right_delay: MovingDelay::new(97.0, 13.0),
            left_dc: DcBlocker::new(sample_rate, 8.0),
            right_dc: DcBlocker::new(sample_rate, 8.0),
        }
    }

    #[inline]
    pub(super) fn sample(&mut self) -> HybridFrame {
        let impact = self.impact.sample();
        let address = self.address.sample();
        self.address_smooth += 0.0007 * (address.value - self.address_smooth);
        let traversal =
            (0.5 + 0.5 * self.traversal.sample() + 0.08 * self.address_smooth).clamp(0.0, 1.0);
        let frame_position = traversal * 3.0;
        let first = (frame_position as usize).min(2);
        let fraction = frame_position - first as f32;
        let mut main = 0.0;
        let mut sum = 0.0;
        for (index, oscillator) in self
            .partials
            .iter_mut()
            .enumerate()
            .take(self.active_partials)
        {
            let amplitude =
                FRAMES[first][index] + fraction * (FRAMES[first + 1][index] - FRAMES[first][index]);
            main += amplitude * oscillator.sample();
            sum += amplitude;
        }
        main /= sum.max(1.0);
        let shadow = self.shadow.sample() * (0.10 + 0.03 * self.address_smooth);
        let left_shadow = self.left_allpass.sample(shadow);
        let right_shadow = self.right_allpass.sample(shadow);
        let (left_low, left_high) = self.left_split.sample(main + left_shadow);
        let (right_low, right_high) = self.right_split.sample(main + right_shadow);
        let left_character = soft_asymmetric(left_high, 1.75) - 0.48 * left_high;
        let right_character = soft_asymmetric(right_high, 2.05) - 0.44 * right_high;
        let onset = 0.16 * impact.cue * (left_character - right_character);
        let body = impact.body;
        let left_base = body * (0.55 * main + 0.19 * left_low + 0.16 * left_character) + onset;
        let right_base = body * (0.55 * main + 0.19 * right_low + 0.16 * right_character) - onset;
        let left_space = self.left_delay.sample(left_base, self.left_motion.sample());
        let right_space = self.right_delay.sample(right_base, self.right_motion.sample());
        HybridFrame {
            left: self
                .left_dc
                .sample(0.67 * left_base + 0.33 * left_space)
                .clamp(-1.0, 1.0),
            right: self
                .right_dc
                .sample(0.67 * right_base + 0.33 * right_space)
                .clamp(-1.0, 1.0),
        }
    }

    pub(super) fn reset(&mut self) {
        for partial in &mut self.partials {
            partial.reset();
        }
        self.shadow.reset();
        self.traversal.reset();
        self.left_motion.reset();
        self.right_motion.reset();
        self.address.reset();
        self.address_smooth = 0.0;
        self.impact.reset();
        self.impact.trigger();
        self.left_split.reset();
        self.right_split.reset();
        self.left_allpass.reset();
        self.right_allpass.reset();
        self.left_delay.reset();
        self.right_delay.reset();
        self.left_dc.reset();
        self.right_dc.reset();
    }
}
