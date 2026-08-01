use crate::control::{MacroId, Normalized, Smoother};
use crate::dsp::{
    character::{CharacterMethod, CharacterMethod::Direct},
    finite_or_zero,
    oscillator::{OscillatorMethod, OscillatorMethod::IntegratedWavetable},
};
use crate::envelope::{Adsr, AdsrConfig};
use crate::preset::{MacroValues, ModelPatchId, Preset, SynthesisModelId};
use crate::synthesis_model::VoiceModel;
use thiserror::Error;

const SMOOTH_SECONDS: f32 = 0.010;
/// Historical lab selections retained for the oscillator comparison CLI.
pub const ENGINE_OSCILLATOR_METHOD: OscillatorMethod = IntegratedWavetable;
/// Historical lab selection retained for the oscillator comparison CLI.
pub const ENGINE_CHARACTER_METHOD: CharacterMethod = Direct;
const ENVELOPE_TIMES: [f32; 17] = [
    0.001, 0.001_857, 0.003_449, 0.006_404, 0.011_89, 0.022_08, 0.041_0, 0.076_2, 0.141_4, 0.262_7,
    0.487_9, 0.906, 1.682, 3.124, 5.80, 10.77, 20.0,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    SetMacro { id: MacroId, value: Normalized },
    AllNotesOff,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedEvent {
    pub sample_offset: usize,
    pub event: Event,
}

impl TimedEvent {
    pub const fn new(sample_offset: usize, event: Event) -> Self {
        Self {
            sample_offset,
            event,
        }
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum EngineError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("left and right buffers must have equal lengths")]
    BufferLengthMismatch,
    #[error("events must be ordered by sample offset")]
    EventsOutOfOrder,
    #[error("event sample offset lies outside the output block")]
    EventOutsideBlock,
    #[error("preset model and patch identities do not match")]
    InvalidModelPatch,
}

#[derive(Debug)]
struct Voice {
    note: u8,
    age: u64,
    model: VoiceModel,
    envelope: Adsr,
    macros: [Smoother; 12],
}

impl Voice {
    fn new(
        sample_rate: f32,
        values: MacroValues,
        model_id: SynthesisModelId,
        patch_id: ModelPatchId,
    ) -> Result<Self, EngineError> {
        let model = VoiceModel::new(sample_rate, model_id, patch_id)?;
        let initial = MacroId::ALL.map(|id| values.get(id).get());
        let macros = initial.map(|value| {
            Smoother::new(value, sample_rate, SMOOTH_SECONDS)
                .map_err(|_| EngineError::InvalidSampleRate)
        });
        let macros = macros
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| EngineError::InvalidSampleRate)?;
        let envelope = Adsr::new(
            sample_rate,
            envelope_config(initial[8], initial[9], initial[10], initial[11]),
        )
        .map_err(|_| EngineError::InvalidSampleRate)?;
        Ok(Self {
            note: 0,
            age: 0,
            model,
            envelope,
            macros,
        })
    }

    fn start(&mut self, note: u8, velocity: f32, age: u64) {
        self.note = note;
        self.age = age;
        self.model.note_on(note, velocity);
        self.envelope.restart();
    }

    #[inline]
    fn next(&mut self) -> f32 {
        let values = self.macros.each_mut().map(Smoother::advance);
        self.model.set_live_controls([
            values[0], values[1], values[2], values[3], values[4], values[5], values[6], values[7],
        ]);
        self.envelope.set_config(envelope_config(
            values[8], values[9], values[10], values[11],
        ));
        if self.envelope.is_idle() {
            return 0.0;
        }
        let envelope = self.envelope.advance();
        let sample = finite_or_zero(self.model.sample() * envelope);
        if self.envelope.is_idle() {
            self.model.reset();
        }
        sample
    }

    fn set_macro_target(&mut self, id: MacroId, value: Normalized) {
        self.macros[id as usize].set_target(value.get());
    }

    fn panic(&mut self) {
        self.envelope.reset();
        self.model.reset();
    }
}

fn envelope_seconds_rt(normalized: f32) -> f32 {
    let position = normalized.clamp(0.0, 1.0) * (ENVELOPE_TIMES.len() - 1) as f32;
    let lower = (position as usize).min(ENVELOPE_TIMES.len() - 2);
    let fraction = position - lower as f32;
    ENVELOPE_TIMES[lower] + fraction * (ENVELOPE_TIMES[lower + 1] - ENVELOPE_TIMES[lower])
}

fn envelope_config(attack: f32, decay: f32, sustain: f32, release: f32) -> AdsrConfig {
    AdsrConfig {
        attack_seconds: envelope_seconds_rt(attack),
        decay_seconds: envelope_seconds_rt(decay),
        sustain_level: sustain.clamp(0.0, 1.0),
        release_seconds: envelope_seconds_rt(release),
    }
}

#[derive(Debug)]
pub struct Engine {
    voices: Vec<Voice>,
    output_gain: f32,
    note_age: u64,
}

impl Engine {
    pub fn new(sample_rate: f32, preset: &Preset) -> Result<Self, EngineError> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EngineError::InvalidSampleRate);
        }
        let mut voices = Vec::with_capacity(preset.voices);
        for _ in 0..preset.voices {
            voices.push(Voice::new(
                sample_rate,
                preset.macros,
                preset.model,
                preset.model_patch,
            )?);
        }
        Ok(Self {
            voices,
            output_gain: preset.output_gain,
            note_age: 0,
        })
    }

    pub fn render_block(
        &mut self,
        events: &[TimedEvent],
        left: &mut [f32],
        right: &mut [f32],
    ) -> Result<(), EngineError> {
        if left.len() != right.len() {
            return Err(EngineError::BufferLengthMismatch);
        }
        if events
            .windows(2)
            .any(|pair| pair[0].sample_offset > pair[1].sample_offset)
        {
            return Err(EngineError::EventsOutOfOrder);
        }
        if events
            .last()
            .is_some_and(|event| event.sample_offset >= left.len())
        {
            return Err(EngineError::EventOutsideBlock);
        }

        let mut event_index = 0;
        for sample_index in 0..left.len() {
            while event_index < events.len() && events[event_index].sample_offset == sample_index {
                self.apply_event(events[event_index].event);
                event_index += 1;
            }
            let mut mixed = 0.0;
            for voice in &mut self.voices {
                mixed += voice.next();
            }
            let sample = finite_or_zero(mixed * self.output_gain);
            left[sample_index] = sample;
            right[sample_index] = sample;
        }
        Ok(())
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::SetMacro { id, value } => {
                for voice in &mut self.voices {
                    voice.set_macro_target(id, value);
                }
            }
            Event::NoteOn { note, velocity } if velocity > 0.0 => {
                self.note_age = self.note_age.wrapping_add(1);
                let index = self
                    .voices
                    .iter()
                    .position(|voice| voice.envelope.is_idle())
                    .or_else(|| {
                        self.voices
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, voice)| voice.age)
                            .map(|(index, _)| index)
                    });
                if let Some(index) = index {
                    self.voices[index].start(note, velocity, self.note_age);
                }
            }
            Event::NoteOn { note, .. } | Event::NoteOff { note } => {
                for voice in &mut self.voices {
                    if voice.note == note && !voice.envelope.is_idle() {
                        voice.model.note_off();
                        voice.envelope.note_off();
                    }
                }
            }
            Event::AllNotesOff => {
                for voice in &mut self.voices {
                    voice.panic();
                }
            }
        }
    }

    pub fn voice_capacity(&self) -> usize {
        self.voices.capacity()
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices
            .iter()
            .filter(|voice| !voice.envelope.is_idle())
            .count()
    }

    pub fn voice_storage_address(&self) -> usize {
        self.voices.as_ptr() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    fn preset(voices: usize) -> Preset {
        Preset::parse(
            &include_str!("../presets/reference.mojsint")
                .replace("voices = 8", &format!("voices = {voices}")),
        )
        .unwrap()
    }

    fn render_with(id: MacroId, value: f32) -> Vec<f32> {
        let mut preset = preset(1);
        match id {
            MacroId::Evolve => preset.macros.evolve = Normalized::new(value).unwrap(),
            MacroId::Shape => preset.macros.shape = Normalized::new(value).unwrap(),
            MacroId::Color => preset.macros.color = Normalized::new(value).unwrap(),
            MacroId::Edge => preset.macros.edge = Normalized::new(value).unwrap(),
            MacroId::Couple => preset.macros.couple = Normalized::new(value).unwrap(),
            MacroId::Motion => preset.macros.motion = Normalized::new(value).unwrap(),
            MacroId::Depth => preset.macros.depth = Normalized::new(value).unwrap(),
            MacroId::Space => preset.macros.space = Normalized::new(value).unwrap(),
            _ => unreachable!(),
        }
        let mut engine = Engine::new(48_000.0, &preset).unwrap();
        let mut left = vec![0.0; 8_192];
        let mut right = vec![0.0; 8_192];
        engine
            .render_block(
                &[TimedEvent::new(
                    0,
                    Event::NoteOn {
                        note: 48,
                        velocity: 0.8,
                    },
                )],
                &mut left,
                &mut right,
            )
            .unwrap();
        left
    }

    fn factory_sources() -> [&'static str; 13] {
        [
            include_str!("../presets/01-full-bass.mojsint"),
            include_str!("../presets/02-full-lead.mojsint"),
            include_str!("../presets/03-full-filter-articulation.mojsint"),
            include_str!("../presets/reference.mojsint"),
            include_str!("../presets/05-matched-linear-mixer.mojsint"),
            include_str!("../presets/06-matched-linear-ladder.mojsint"),
            include_str!("../presets/07-matched-no-drift-or-feedback.mojsint"),
            include_str!("../presets/08-six-op-bell-metal.mojsint"),
            include_str!("../presets/09-six-op-fractured-metal.mojsint"),
            include_str!("../presets/10-six-op-electric-piano-mallet.mojsint"),
            include_str!("../presets/11-six-op-glass-wood.mojsint"),
            include_str!("../presets/12-six-op-brass-bass.mojsint"),
            include_str!("../presets/13-six-op-mechanical-stab.mojsint"),
        ]
    }

    #[test]
    fn renders_timed_stereo_and_panics_to_silence() {
        let mut engine = Engine::new(48_000.0, &preset(2)).unwrap();
        let mut left = [0.0; 256];
        let mut right = [0.0; 256];
        engine
            .render_block(
                &[TimedEvent::new(
                    2,
                    Event::NoteOn {
                        note: 60,
                        velocity: 1.0,
                    },
                )],
                &mut left,
                &mut right,
            )
            .unwrap();
        assert_eq!(&left[..2], &[0.0, 0.0]);
        assert_eq!(left, right);
        assert!(left.iter().all(|sample| sample.is_finite()));
        engine
            .render_block(
                &[TimedEvent::new(0, Event::AllNotesOff)],
                &mut left,
                &mut right,
            )
            .unwrap();
        assert!(left.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn all_eight_timbral_controls_are_measurably_active() {
        for id in MacroId::TIMBRAL {
            let low = render_with(id, 0.0);
            let high = render_with(id, 1.0);
            let rms = (low
                .iter()
                .zip(high)
                .skip(1024)
                .map(|(a, b)| f64::from(a - b).powi(2))
                .sum::<f64>()
                / (low.len() - 1024) as f64)
                .sqrt();
            assert!(rms > 1.0e-5, "{id:?} rms difference={rms}");
        }
    }

    #[test]
    fn thirteen_factory_starting_points_are_finite_and_pairwise_distinct() {
        let renders = factory_sources().map(|source| {
            let mut preset = Preset::parse(source).unwrap();
            preset.voices = 1;
            let mut engine = Engine::new(48_000.0, &preset).unwrap();
            let mut left = vec![0.0; 16_384];
            let mut right = vec![0.0; left.len()];
            engine
                .render_block(
                    &[TimedEvent::new(
                        0,
                        Event::NoteOn {
                            note: 48,
                            velocity: 0.8,
                        },
                    )],
                    &mut left,
                    &mut right,
                )
                .unwrap();
            assert_eq!(left, right);
            assert!(left.iter().all(|sample| sample.is_finite()));
            left
        });

        for left in 0..renders.len() {
            for right in left + 1..renders.len() {
                let residual = renders[left]
                    .iter()
                    .zip(&renders[right])
                    .skip(1_024)
                    .map(|(a, b)| f64::from(a - b).powi(2))
                    .sum::<f64>()
                    / (renders[left].len() - 1_024) as f64;
                assert!(
                    residual.sqrt() > 1.0e-6,
                    "factory presets {left} and {right} collapsed"
                );
            }
        }
    }

    #[test]
    fn rapid_control_movement_is_finite_bounded_and_allocation_free() {
        let mut engine = Engine::new(48_000.0, &preset(2)).unwrap();
        let mut events = [TimedEvent::new(0, Event::AllNotesOff); 25];
        events[0] = TimedEvent::new(
            0,
            Event::NoteOn {
                note: 84,
                velocity: 1.0,
            },
        );
        for (index, id) in MacroId::ALL.into_iter().enumerate() {
            events[index * 2 + 1] = TimedEvent::new(
                index * 4 + 1,
                Event::SetMacro {
                    id,
                    value: Normalized::new(1.0).unwrap(),
                },
            );
            events[index * 2 + 2] = TimedEvent::new(
                index * 4 + 2,
                Event::SetMacro {
                    id,
                    value: Normalized::new(0.0).unwrap(),
                },
            );
        }
        let mut left = [0.0; 128];
        let mut right = [0.0; 128];
        assert_no_alloc(|| {
            engine.render_block(&events, &mut left, &mut right).unwrap();
        });
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(left.iter().all(|sample| sample.abs() < 4.0));
        assert_eq!(engine.voice_capacity(), 2);
    }
}
