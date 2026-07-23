mod cross_coupled;
mod dual_body;
mod primitives;
mod spectral_shadow;

use cross_coupled::CrossCoupledMachine;
use dual_body::DualResonantBody;
use spectral_shadow::SpectralShadow;
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
// These disposable research voices deliberately keep their delay/resonator
// storage inline. Boxing the largest variants would trade predictable fixed
// state for heap allocation and pointer indirection at this DSP boundary.
#[allow(clippy::large_enum_variant)]
enum HybridState {
    CrossCoupledMachine(CrossCoupledMachine),
    SpectralShadow(SpectralShadow),
    DualResonantBody(DualResonantBody),
}

#[derive(Debug)]
pub struct HybridVoice {
    family: HybridFamily,
    state: HybridState,
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
        let state = match family {
            HybridFamily::CrossCoupledMachine => HybridState::CrossCoupledMachine(
                CrossCoupledMachine::new(sample_rate, frequency_hz, seed),
            ),
            HybridFamily::SpectralShadow => {
                HybridState::SpectralShadow(SpectralShadow::new(sample_rate, frequency_hz, seed))
            }
            HybridFamily::DualResonantBody => HybridState::DualResonantBody(DualResonantBody::new(
                sample_rate,
                frequency_hz,
                seed,
            )),
        };
        Ok(Self { family, state })
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        match &mut self.state {
            HybridState::CrossCoupledMachine(voice) => voice.sample(),
            HybridState::SpectralShadow(voice) => voice.sample(),
            HybridState::DualResonantBody(voice) => voice.sample(),
        }
    }

    pub fn reset(&mut self) {
        match &mut self.state {
            HybridState::CrossCoupledMachine(voice) => voice.reset(),
            HybridState::SpectralShadow(voice) => voice.reset(),
            HybridState::DualResonantBody(voice) => voice.reset(),
        }
    }

    pub fn family(&self) -> HybridFamily {
        self.family
    }

    pub fn movement_rates_hz(&self) -> [f32; 3] {
        match self.family {
            HybridFamily::CrossCoupledMachine => [0.61, 0.37, 0.43],
            HybridFamily::SpectralShadow => [0.19, 0.31, 0.47],
            HybridFamily::DualResonantBody => [0.17, 0.23, 0.41],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HybridFamily, HybridVoice};
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn hybrid_api_rejects_invalid_configuration_and_resets_deterministically() {
        assert!(
            HybridVoice::new(HybridFamily::CrossCoupledMachine, 0.0, 220.0, 0x1234_5678).is_err()
        );
        assert!(
            HybridVoice::new(
                HybridFamily::CrossCoupledMachine,
                48_000.0,
                24_000.0,
                0x1234_5678
            )
            .is_err()
        );

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
            AllPass, DcBlocker, ImpactEnvelope, MovingDelay, OnePoleSplit, PhaseOsc, RegisterOsc,
            soft_asymmetric, soft_clip,
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

    fn assert_complete_stereo_voice(family: HybridFamily) -> u64 {
        let mut voice = HybridVoice::new(family, 48_000.0, 110.0, 0x51a7_9e3d).unwrap();
        let mut left_energy = 0.0_f64;
        let mut right_energy = 0.0_f64;
        let mut mid_energy = 0.0_f64;
        let mut side_energy = 0.0_f64;
        let mut cross = 0.0_f64;
        let mut dc = 0.0_f64;
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for _ in 0..96_000 {
            let frame = assert_no_alloc(|| voice.sample());
            assert!(frame.left.is_finite() && frame.right.is_finite());
            assert!(frame.left.abs() <= 1.0 && frame.right.abs() <= 1.0);
            let left = f64::from(frame.left);
            let right = f64::from(frame.right);
            let mid = 0.5 * (left + right);
            let side = 0.5 * (left - right);
            left_energy += left * left;
            right_energy += right * right;
            mid_energy += mid * mid;
            side_energy += side * side;
            cross += left * right;
            dc += mid;
            hash ^= u64::from(frame.left.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            hash ^= u64::from(frame.right.to_bits());
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        let correlation = cross / (left_energy * right_energy).sqrt();
        assert!(mid_energy > 10.0, "family={family:?} mid={mid_energy}");
        assert!(
            side_energy / mid_energy > 0.005,
            "family={family:?} side/mid={}",
            side_energy / mid_energy
        );
        assert!(correlation < 0.995, "family={family:?} corr={correlation}");
        assert!(dc.abs() / 96_000.0 < 5e-4, "family={family:?} dc={dc}");
        voice.reset();
        let reset = voice.sample();
        let mut fresh = HybridVoice::new(family, 48_000.0, 110.0, 0x51a7_9e3d).unwrap();
        assert_eq!(reset, fresh.sample());
        hash
    }

    #[test]
    fn cross_coupled_machine_is_a_complete_stereo_voice() {
        assert_complete_stereo_voice(HybridFamily::CrossCoupledMachine);
        let rates = HybridVoice::new(HybridFamily::CrossCoupledMachine, 48_000.0, 110.0, 7)
            .unwrap()
            .movement_rates_hz();
        assert!(rates[0] != rates[1] && rates[1] != rates[2]);
    }

    #[test]
    fn spectral_shadow_is_a_complete_stereo_voice() {
        assert_complete_stereo_voice(HybridFamily::SpectralShadow);
    }

    #[test]
    fn dual_resonant_body_is_a_complete_stereo_voice() {
        assert_complete_stereo_voice(HybridFamily::DualResonantBody);
    }

    #[test]
    fn complete_voice_topologies_are_deterministically_distinct() {
        let hashes = [
            assert_complete_stereo_voice(HybridFamily::CrossCoupledMachine),
            assert_complete_stereo_voice(HybridFamily::SpectralShadow),
            assert_complete_stereo_voice(HybridFamily::DualResonantBody),
        ];
        assert_ne!(hashes[0], hashes[1]);
        assert_ne!(hashes[1], hashes[2]);
        assert_ne!(hashes[0], hashes[2]);
    }
}
