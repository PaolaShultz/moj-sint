mod primitives;

use primitives::{ImpactEnvelope, PhaseOsc};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HybridFamily {
    CrossCoupledMachine,
    SpectralShadow,
    DualResonantBody,
}

impl HybridFamily {
    pub const ALL: [Self; 3] = [
        Self::CrossCoupledMachine,
        Self::SpectralShadow,
        Self::DualResonantBody,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HybridFrame {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum HybridError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("frequency must be finite, positive, and below Nyquist")]
    InvalidFrequency,
}

#[derive(Debug)]
struct ScaffoldVoice {
    left: PhaseOsc,
    right: PhaseOsc,
    impact: ImpactEnvelope,
}

impl ScaffoldVoice {
    fn new(sample_rate: f32, frequency_hz: f32, seed: u32) -> Self {
        let phase = (seed & 0xffff) as f32 / 65_536.0;
        let mut impact = ImpactEnvelope::new(sample_rate);
        impact.trigger();
        Self {
            left: PhaseOsc::new(sample_rate, frequency_hz, phase),
            right: PhaseOsc::new(sample_rate, frequency_hz, phase + 0.037),
            impact,
        }
    }

    fn sample(&mut self) -> HybridFrame {
        let impact = self.impact.sample();
        let gain = 0.08 * impact.cue + 0.42 * impact.body;
        HybridFrame {
            left: gain * self.left.sample(),
            right: gain * self.right.sample(),
        }
    }

    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.impact.reset();
        self.impact.trigger();
    }
}

#[derive(Debug)]
pub struct HybridVoice {
    family: HybridFamily,
    state: ScaffoldVoice,
}

impl HybridVoice {
    pub fn new(
        family: HybridFamily,
        sample_rate: f32,
        frequency_hz: f32,
        seed: u32,
    ) -> Result<Self, HybridError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(HybridError::InvalidSampleRate);
        }
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 || frequency_hz >= sample_rate * 0.5 {
            return Err(HybridError::InvalidFrequency);
        }
        Ok(Self {
            family,
            state: ScaffoldVoice::new(sample_rate, frequency_hz, seed),
        })
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        self.state.sample()
    }

    pub fn reset(&mut self) {
        self.state.reset();
    }

    pub fn family(&self) -> HybridFamily {
        self.family
    }
}

#[cfg(test)]
mod tests {
    use super::{HybridFamily, HybridVoice};
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn hybrid_api_rejects_invalid_configuration_and_resets_deterministically() {
        assert!(HybridVoice::new(
            HybridFamily::CrossCoupledMachine,
            0.0,
            220.0,
            0x1234_5678
        )
        .is_err());
        assert!(HybridVoice::new(
            HybridFamily::CrossCoupledMachine,
            48_000.0,
            24_000.0,
            0x1234_5678
        )
        .is_err());

        let mut voice = HybridVoice::new(
            HybridFamily::CrossCoupledMachine,
            48_000.0,
            220.0,
            0x1234_5678,
        )
        .unwrap();
        let first = voice.sample();
        for _ in 0..512 {
            let frame = assert_no_alloc(|| voice.sample());
            assert!(frame.left.is_finite() && frame.right.is_finite());
            assert!(frame.left.abs() <= 1.0 && frame.right.abs() <= 1.0);
        }
        voice.reset();
        assert_eq!(first, voice.sample());
    }

    #[test]
    fn prepared_primitives_are_bounded_explicit_and_allocation_free() {
        use super::primitives::{
            AllPass, DcBlocker, ImpactEnvelope, MovingDelay, OnePoleSplit, PhaseOsc,
            RegisterOsc, soft_asymmetric, soft_clip,
        };

        let mut oscillator = PhaseOsc::new(48_000.0, 220.0, 0.125);
        let mut register = RegisterOsc::new(12, 48_000.0, 110.0, 0x531, 3);
        let mut envelope = ImpactEnvelope::new(48_000.0);
        let mut split = OnePoleSplit::new(48_000.0, 420.0);
        let mut allpass = AllPass::new(0.41);
        let mut delay = MovingDelay::<512>::new(64.0, 8.0);
        let mut dc = DcBlocker::new(48_000.0, 8.0);
        envelope.trigger();

        for _ in 0..4_096 {
            assert_no_alloc(|| {
                let source = oscillator.sample();
                let machine = register.sample();
                let impact = envelope.sample();
                let (low, high) = split.sample(source);
                assert!(((low + high) - source).abs() < 1e-6);
                let shifted = allpass.sample(low);
                let delayed = delay.sample(shifted, machine.value);
                let output = dc.sample(soft_clip(delayed + impact.cue));
                assert!(output.is_finite() && output.abs() <= 1.0);
                assert!(soft_asymmetric(high, 1.7).is_finite());
            });
        }
        assert!(register.width() == 12);
        assert!(delay.minimum_delay() >= 2.0);
        assert!(delay.maximum_delay() < 512.0);
    }
}
