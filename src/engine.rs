use crate::dsp::{finite_or_zero, oscillator::SineOscillator};
use crate::envelope::{Adsr, AdsrConfig};
use crate::preset::Preset;
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8 },
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
    oscillator: SineOscillator,
    envelope: Adsr,
}

impl Voice {
    fn new(sample_rate: f32, envelope: AdsrConfig) -> Result<Self, EngineError> {
        let oscillator =
            SineOscillator::new(sample_rate).map_err(|_| EngineError::InvalidSampleRate)?;
        let envelope =
            Adsr::new(sample_rate, envelope).map_err(|_| EngineError::InvalidSampleRate)?;
        Ok(Self {
            note: 0,
            velocity: 0.0,
            age: 0,
            oscillator,
            envelope,
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
        self.envelope.restart();
    }

    fn next(&mut self) -> f32 {
        if self.envelope.is_idle() {
            return 0.0;
        }
        let frequency = 440.0 * 2.0_f32.powf((f32::from(self.note) - 69.0) / 12.0);
        finite_or_zero(self.oscillator.next(frequency) * self.envelope.next() * self.velocity)
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
            voices.push(Voice::new(sample_rate, preset.envelope)?);
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
        assert_eq!(&left[..=2], &[0.0, 0.0, 0.0]);
        assert!(left[3..].iter().any(|sample| sample.abs() > 0.0));
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
}
