use crate::control::{MacroId, Normalized, Smoother};
use crate::dsp::{
    finite_or_zero,
    harmonic_selector::ThreePhaseBank,
    oscillator::{BandlimitedOscillator, OscillatorMethod},
};
use crate::envelope::{Adsr, AdsrConfig};
use crate::preset::{MacroValues, Preset};
use thiserror::Error;

pub const ENGINE_OSCILLATOR_METHOD: OscillatorMethod = OscillatorMethod::IntegratedWavetable;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
    SetMacro { id: MacroId, value: Normalized },
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
}

#[derive(Debug)]
struct Voice {
    note: u8,
    velocity: f32,
    age: u64,
    oscillator: BandlimitedOscillator,
    harmonic_selector: ThreePhaseBank,
    envelope: Adsr,
    shape: Smoother,
    color: Smoother,
    edge: Smoother,
    couple: Smoother,
    color_lowpass: f32,
}

impl Voice {
    fn new(
        sample_rate: f32,
        envelope: AdsrConfig,
        macros: MacroValues,
    ) -> Result<Self, EngineError> {
        let oscillator = BandlimitedOscillator::new(sample_rate, ENGINE_OSCILLATOR_METHOD)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        let harmonic_selector =
            ThreePhaseBank::new(sample_rate).map_err(|_| EngineError::InvalidSampleRate)?;
        let envelope =
            Adsr::new(sample_rate, envelope).map_err(|_| EngineError::InvalidSampleRate)?;
        let shape = Smoother::new(macros.shape.get(), sample_rate, 0.01)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        let color = Smoother::new(macros.color.get(), sample_rate, 0.01)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        let edge = Smoother::new(macros.edge.get(), sample_rate, 0.01)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        let couple = Smoother::new(macros.couple.get(), sample_rate, 0.01)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        Ok(Self {
            note: 0,
            velocity: 0.0,
            age: 0,
            oscillator,
            harmonic_selector,
            envelope,
            shape,
            color,
            edge,
            couple,
            color_lowpass: 0.0,
        })
    }

    fn start(&mut self, note: u8, velocity: f32, age: u64) {
        self.note = note;
        self.velocity = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.age = age;
        self.oscillator.reset();
        let frequency = 440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0);
        self.oscillator.set_frequency(frequency);
        self.harmonic_selector.set_frequency(frequency);
        self.color_lowpass = 0.0;
        self.envelope.restart();
    }

    fn next(&mut self) -> f32 {
        let shape = self.shape.advance();
        let color = self.color.advance();
        let edge = self.edge.advance();
        let couple = self.couple.advance();
        if self.envelope.is_idle() {
            return 0.0;
        }
        let oscillator_sample = self.oscillator.sample(shape);
        let color_coefficient =
            (self.oscillator.phase_increment() * (8.0 + 120.0 * color * color)).clamp(0.001, 0.75);
        self.color_lowpass += color_coefficient * (oscillator_sample - self.color_lowpass);
        let colored = self.color_lowpass + color * (oscillator_sample - self.color_lowpass);
        let harmonic = self.harmonic_selector.sample();
        let selected = harmonic.fundamental + edge * (harmonic.third - harmonic.fundamental);
        let coupled = colored + couple * (selected - colored);
        let shape_compensation = 1.0 + 4.0 * shape * (1.0 - shape);
        let color_compensation = 1.15 - 0.15 * color;
        let selector_compensation =
            (1.0 + 0.4 * couple * (1.0 - couple)) * (1.0 + 0.2 * couple * edge * (1.0 - edge));
        finite_or_zero(
            coupled
                * shape_compensation
                * color_compensation
                * selector_compensation
                * self.envelope.advance()
                * self.velocity,
        )
    }

    fn set_macro_target(&mut self, id: MacroId, value: Normalized) {
        match id {
            MacroId::Shape => self.shape.set_target(value.get()),
            MacroId::Color => self.color.set_target(value.get()),
            MacroId::Edge => self.edge.set_target(value.get()),
            MacroId::Couple => self.couple.set_target(value.get()),
            _ => {}
        }
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
            voices.push(Voice::new(sample_rate, preset.envelope, preset.macros)?);
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
                        voice.envelope.note_off();
                    }
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
    use crate::control::{MacroId, Normalized};
    use crate::preset::Preset;

    #[global_allocator]
    static ALLOCATOR: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

    fn preset(voices: usize) -> Preset {
        let source = include_str!("../presets/reference.mojsint")
            .replace("voices = 8", &format!("voices = {voices}"))
            .replace("attack_seconds = 0.01", "attack_seconds = 0.001");
        Preset::parse(&source).unwrap()
    }

    #[test]
    fn applies_note_events_at_sample_offsets_and_renders_stereo() {
        let mut engine = Engine::new(1_000.0, &preset(2)).unwrap();
        let events = [TimedEvent::new(
            2,
            Event::NoteOn {
                note: 69,
                velocity: 1.0,
            },
        )];
        let mut left = [0.0; 12];
        let mut right = [0.0; 12];
        engine.render_block(&events, &mut left, &mut right).unwrap();
        assert_eq!(&left[..2], &[0.0, 0.0]);
        assert!(left[2..].iter().any(|sample| sample.abs() > 0.0));
        assert_eq!(left, right);
        assert!(left.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn fixed_voice_capacity_steals_without_growing() {
        let mut engine = Engine::new(48_000.0, &preset(2)).unwrap();
        let events = [
            TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            ),
            TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 64,
                    velocity: 1.0,
                },
            ),
            TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 67,
                    velocity: 1.0,
                },
            ),
        ];
        let mut left = [0.0; 16];
        let mut right = [0.0; 16];
        engine.render_block(&events, &mut left, &mut right).unwrap();
        assert_eq!(engine.voice_capacity(), 2);
        assert_eq!(engine.active_voice_count(), 2);
    }

    #[test]
    fn rejects_invalid_event_order_and_buffer_shapes_before_rendering() {
        let mut engine = Engine::new(48_000.0, &preset(2)).unwrap();
        let mut left = [0.0; 8];
        let mut short_right = [0.0; 7];
        assert!(
            engine
                .render_block(&[], &mut left, &mut short_right)
                .is_err()
        );
        let mut right = [0.0; 8];
        let reversed = [
            TimedEvent::new(4, Event::NoteOff { note: 60 }),
            TimedEvent::new(
                2,
                Event::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            ),
        ];
        assert!(
            engine
                .render_block(&reversed, &mut left, &mut right)
                .is_err()
        );
    }

    #[test]
    fn render_keeps_preallocated_voice_storage_stable() {
        let mut engine = Engine::new(48_000.0, &preset(4)).unwrap();
        let storage = engine.voice_storage_address();
        let mut left = [0.0; 64];
        let mut right = [0.0; 64];
        for _ in 0..100 {
            assert_no_alloc::assert_no_alloc(|| {
                engine.render_block(&[], &mut left, &mut right).unwrap();
            });
            assert_eq!(engine.voice_storage_address(), storage);
        }
    }

    #[test]
    fn shape_and_color_change_output_across_most_of_their_travel() {
        fn render_macro(id: MacroId, value: f32) -> Vec<f32> {
            let mut preset = preset(1);
            match id {
                MacroId::Shape => preset.macros.shape = Normalized::new(value).unwrap(),
                MacroId::Color => preset.macros.color = Normalized::new(value).unwrap(),
                _ => unreachable!(),
            }
            let mut engine = Engine::new(48_000.0, &preset).unwrap();
            let events = [TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            )];
            let mut left = vec![0.0; 4_096];
            let mut right = vec![0.0; 4_096];
            engine.render_block(&events, &mut left, &mut right).unwrap();
            left
        }

        for id in [MacroId::Shape, MacroId::Color] {
            let renders: Vec<_> = [0.0, 0.25, 0.5, 0.75, 1.0]
                .into_iter()
                .map(|value| render_macro(id, value))
                .collect();
            for (travel_index, pair) in renders.windows(2).enumerate() {
                let difference_energy = pair[0]
                    .iter()
                    .zip(&pair[1])
                    .skip(512)
                    .map(|(left, right)| f64::from(left - right).powi(2))
                    .sum::<f64>();
                let difference_rms = (difference_energy / (pair[0].len() - 512) as f64).sqrt();
                assert!(
                    difference_rms > 0.005,
                    "{id:?} travel segment {travel_index} was nearly inert: {difference_rms}"
                );
            }
            for render in renders {
                let peak = render
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
                assert!(peak > 0.01 && peak <= 0.35, "{id:?} peak={peak}");
                assert!(render.iter().all(|sample| sample.is_finite()));
            }
        }
    }

    #[test]
    fn edge_and_couple_change_output_across_most_of_their_travel() {
        fn render_macro(id: MacroId, value: f32) -> Vec<f32> {
            let mut preset = preset(1);
            match id {
                MacroId::Edge => preset.macros.edge = Normalized::new(value).unwrap(),
                MacroId::Couple => preset.macros.couple = Normalized::new(value).unwrap(),
                _ => unreachable!(),
            }
            let mut engine = Engine::new(48_000.0, &preset).unwrap();
            let events = [TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 60,
                    velocity: 1.0,
                },
            )];
            let mut left = vec![0.0; 4_096];
            let mut right = vec![0.0; 4_096];
            engine.render_block(&events, &mut left, &mut right).unwrap();
            left
        }

        for id in [MacroId::Edge, MacroId::Couple] {
            let renders: Vec<_> = [0.0, 0.25, 0.5, 0.75, 1.0]
                .into_iter()
                .map(|value| render_macro(id, value))
                .collect();
            for (travel_index, pair) in renders.windows(2).enumerate() {
                let difference_energy = pair[0]
                    .iter()
                    .zip(&pair[1])
                    .skip(512)
                    .map(|(left, right)| f64::from(left - right).powi(2))
                    .sum::<f64>();
                let difference_rms = (difference_energy / (pair[0].len() - 512) as f64).sqrt();
                assert!(
                    difference_rms > 0.005,
                    "{id:?} travel segment {travel_index} was nearly inert: {difference_rms}"
                );
            }
            for render in renders {
                let peak = render
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
                assert!(peak > 0.01 && peak <= 0.45, "{id:?} peak={peak}");
                assert!(render.iter().all(|sample| sample.is_finite()));
            }
        }
    }

    #[test]
    fn timed_macro_changes_are_smoothed() {
        let mut preset = preset(1);
        preset.macros.shape = Normalized::new(0.0).unwrap();
        let mut engine = Engine::new(48_000.0, &preset).unwrap();
        let mut left = [0.0; 64];
        let mut right = [0.0; 64];
        engine
            .render_block(
                &[TimedEvent::new(
                    0,
                    Event::NoteOn {
                        note: 60,
                        velocity: 1.0,
                    },
                )],
                &mut left,
                &mut right,
            )
            .unwrap();
        let mut one_left = [0.0; 1];
        let mut one_right = [0.0; 1];
        engine
            .render_block(
                &[TimedEvent::new(
                    0,
                    Event::SetMacro {
                        id: MacroId::Shape,
                        value: Normalized::new(1.0).unwrap(),
                    },
                )],
                &mut one_left,
                &mut one_right,
            )
            .unwrap();
        let smoothed = engine.voices[0].shape.current();
        assert!(smoothed > 0.0 && smoothed < 0.01, "shape={smoothed}");
        let mut settle_left = [0.0; 4_800];
        let mut settle_right = [0.0; 4_800];
        engine
            .render_block(&[], &mut settle_left, &mut settle_right)
            .unwrap();
        assert!((engine.voices[0].shape.current() - 1.0).abs() < 1.0e-4);
    }

    #[test]
    fn timed_harmonic_selector_changes_are_smoothed() {
        let mut preset = preset(1);
        preset.macros.edge = Normalized::new(0.0).unwrap();
        preset.macros.couple = Normalized::new(0.0).unwrap();
        let mut engine = Engine::new(48_000.0, &preset).unwrap();
        let mut left = [0.0; 1];
        let mut right = [0.0; 1];
        engine
            .render_block(
                &[
                    TimedEvent::new(
                        0,
                        Event::NoteOn {
                            note: 60,
                            velocity: 1.0,
                        },
                    ),
                    TimedEvent::new(
                        0,
                        Event::SetMacro {
                            id: MacroId::Edge,
                            value: Normalized::new(1.0).unwrap(),
                        },
                    ),
                    TimedEvent::new(
                        0,
                        Event::SetMacro {
                            id: MacroId::Couple,
                            value: Normalized::new(1.0).unwrap(),
                        },
                    ),
                ],
                &mut left,
                &mut right,
            )
            .unwrap();
        assert!(engine.voices[0].edge.current() > 0.0 && engine.voices[0].edge.current() < 0.01);
        assert!(
            engine.voices[0].couple.current() > 0.0 && engine.voices[0].couple.current() < 0.01
        );
    }

    #[test]
    fn rapid_macro_changes_remain_finite_bounded_and_allocation_free() {
        let mut engine = Engine::new(48_000.0, &preset(2)).unwrap();
        let events = [
            TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 84,
                    velocity: 1.0,
                },
            ),
            TimedEvent::new(
                8,
                Event::SetMacro {
                    id: MacroId::Shape,
                    value: Normalized::new(0.0).unwrap(),
                },
            ),
            TimedEvent::new(
                16,
                Event::SetMacro {
                    id: MacroId::Color,
                    value: Normalized::new(1.0).unwrap(),
                },
            ),
            TimedEvent::new(
                24,
                Event::SetMacro {
                    id: MacroId::Shape,
                    value: Normalized::new(1.0).unwrap(),
                },
            ),
            TimedEvent::new(
                32,
                Event::SetMacro {
                    id: MacroId::Color,
                    value: Normalized::new(0.0).unwrap(),
                },
            ),
            TimedEvent::new(
                40,
                Event::SetMacro {
                    id: MacroId::Edge,
                    value: Normalized::new(1.0).unwrap(),
                },
            ),
            TimedEvent::new(
                48,
                Event::SetMacro {
                    id: MacroId::Couple,
                    value: Normalized::new(1.0).unwrap(),
                },
            ),
            TimedEvent::new(
                56,
                Event::SetMacro {
                    id: MacroId::Edge,
                    value: Normalized::new(0.0).unwrap(),
                },
            ),
            TimedEvent::new(
                64,
                Event::SetMacro {
                    id: MacroId::Couple,
                    value: Normalized::new(0.0).unwrap(),
                },
            ),
        ];
        let mut left = [0.0; 128];
        let mut right = [0.0; 128];
        assert_no_alloc::assert_no_alloc(|| {
            engine.render_block(&events, &mut left, &mut right).unwrap();
        });
        assert!(left.iter().all(|sample| sample.is_finite()));
        assert!(left.iter().all(|sample| sample.abs() <= 0.35));
        assert_eq!(left, right);
    }
}
