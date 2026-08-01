use crate::dsp::six_op_pm::operator::SharedSineTable;
use crate::dsp::six_op_pm::{SineTable, SixOpVoice, VoiceError};
use crate::preset::SixOpPatchId;
use crate::six_op_pm::ListeningPatch;

#[derive(Clone, Debug)]
pub struct LiveSixOpVoice {
    sample_rate: f32,
    patch: SixOpPatchId,
    sine_table: SharedSineTable,
    voice: SixOpVoice,
    controls: [f32; 8],
    note: u8,
    velocity: f32,
}

impl LiveSixOpVoice {
    pub fn new(sample_rate: f32, patch: SixOpPatchId) -> Result<Self, VoiceError> {
        let sine_table: SharedSineTable = SineTable::new().into();
        let note = 69;
        let velocity = 1.0;
        let controls = [0.5; 8];
        let mut voice = SixOpVoice::new_with_sine_table(
            listening_patch(patch).patch(),
            sample_rate,
            note,
            velocity,
            sine_table.clone(),
        )?;
        voice.set_live_controls(controls, note, velocity);
        Ok(Self {
            sample_rate,
            patch,
            sine_table,
            voice,
            controls,
            note,
            velocity,
        })
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) -> Result<(), VoiceError> {
        let mut voice = SixOpVoice::new_with_sine_table(
            self.prepared_patch(),
            self.sample_rate,
            note,
            velocity,
            self.sine_table.clone(),
        )?;
        voice.set_live_controls(self.controls, note, velocity);
        voice.note_on();
        self.note = note;
        self.velocity = velocity;
        self.voice = voice;
        Ok(())
    }

    pub fn note_off(&mut self) {
        self.voice.note_off();
    }

    pub fn set_live_controls(&mut self, values: [f32; 8]) {
        self.controls = values.map(|value| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.5
            }
        });
        self.voice
            .set_live_controls(self.controls, self.note, self.velocity);
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        self.voice.sample()
    }

    pub fn reset(&mut self) {
        self.voice.reset();
    }

    fn prepared_patch(&self) -> crate::dsp::six_op_pm::SixOpPatch {
        let mut patch = listening_patch(self.patch).patch();
        let time_scale = 2.0_f32.powf((self.controls[3] - 0.5) * 2.0);
        for operator in &mut patch.operators {
            for seconds in &mut operator.envelope.seconds {
                *seconds *= time_scale;
            }
        }
        patch
    }
}

const fn listening_patch(patch: SixOpPatchId) -> ListeningPatch {
    match patch {
        SixOpPatchId::BellMetal => ListeningPatch::BellMetal,
        SixOpPatchId::FracturedMetal => ListeningPatch::FracturedMetal,
        SixOpPatchId::ElectricPianoMallet => ListeningPatch::ElectricPianoMallet,
        SixOpPatchId::GlassWood => ListeningPatch::GlassWood,
        SixOpPatchId::BrassBass => ListeningPatch::BrassBass,
        SixOpPatchId::MechanicalStab => ListeningPatch::MechanicalStab,
    }
}

#[cfg(test)]
mod tests {
    use assert_no_alloc::assert_no_alloc;

    use super::*;

    fn render(voice: &mut LiveSixOpVoice, samples: usize) -> Vec<f32> {
        (0..samples).map(|_| voice.sample()).collect()
    }

    #[test]
    fn neutral_live_voice_is_deterministic_finite_and_resettable() {
        let mut voice = LiveSixOpVoice::new(48_000.0, SixOpPatchId::BellMetal).unwrap();
        voice.note_on(60, 0.8).unwrap();
        let first = render(&mut voice, 4_096);
        voice.reset();
        voice.note_on(60, 0.8).unwrap();
        let second = render(&mut voice, 4_096);
        assert_eq!(first, second);
        assert!(first.iter().all(|sample| sample.is_finite()));
        assert!(first.iter().any(|sample| sample.abs() > 1.0e-6));
    }

    #[test]
    fn every_live_control_changes_the_authored_voice() {
        for control in 0..8 {
            let mut low = LiveSixOpVoice::new(48_000.0, SixOpPatchId::FracturedMetal).unwrap();
            let mut high = low.clone();
            let mut low_values = [0.5; 8];
            let mut high_values = [0.5; 8];
            low_values[control] = 0.0;
            high_values[control] = 1.0;
            low.set_live_controls(low_values);
            high.set_live_controls(high_values);
            low.note_on(72, 0.76).unwrap();
            high.note_on(72, 0.76).unwrap();
            let low = render(&mut low, 8_192);
            let high = render(&mut high, 8_192);
            let residual = low
                .iter()
                .zip(high)
                .map(|(left, right)| f64::from(left - right).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(residual > 1.0e-5, "control {control} residual={residual}");
        }
    }

    #[test]
    fn note_control_and_sample_paths_do_not_allocate() {
        let mut voice = LiveSixOpVoice::new(48_000.0, SixOpPatchId::MechanicalStab).unwrap();
        assert_no_alloc(|| {
            voice.set_live_controls([0.7, 0.4, 0.8, 0.6, 0.3, 0.9, 0.2, 0.75]);
            voice.note_on(48, 0.9).unwrap();
            for _ in 0..4_096 {
                std::hint::black_box(voice.sample());
            }
            voice.note_off();
            voice.reset();
        });
    }

    #[test]
    fn operator_decay_prepares_authored_envelope_times_for_the_next_note() {
        let mut short = LiveSixOpVoice::new(48_000.0, SixOpPatchId::BrassBass).unwrap();
        let mut long = short.clone();
        let authored = short.prepared_patch();
        let mut short_values = [0.5; 8];
        let mut long_values = [0.5; 8];
        short_values[3] = 0.0;
        long_values[3] = 1.0;
        short.set_live_controls(short_values);
        long.set_live_controls(long_values);
        let short = short.prepared_patch();
        let long = long.prepared_patch();

        for operator in 0..6 {
            for stage in 0..4 {
                assert_eq!(
                    short.operators[operator].envelope.seconds[stage],
                    authored.operators[operator].envelope.seconds[stage] * 0.5
                );
                assert_eq!(
                    long.operators[operator].envelope.seconds[stage],
                    authored.operators[operator].envelope.seconds[stage] * 2.0
                );
            }
        }
    }
}
