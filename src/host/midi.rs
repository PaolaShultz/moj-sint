use crate::control::{MacroId, Normalized};
use crate::engine::Event;
use alsa::seq::{EvCtrl, EvNote, Event as AlsaEvent, EventType};

pub fn translate(event: &AlsaEvent<'_>) -> Option<Event> {
    match event.get_type() {
        EventType::Noteon => {
            let note = event.get_data::<EvNote>()?;
            Some(if note.velocity == 0 {
                Event::NoteOff { note: note.note }
            } else {
                Event::NoteOn {
                    note: note.note,
                    velocity: f32::from(note.velocity) / 127.0,
                }
            })
        }
        EventType::Noteoff => {
            let note = event.get_data::<EvNote>()?;
            Some(Event::NoteOff { note: note.note })
        }
        EventType::Controller => {
            let control = event.get_data::<EvCtrl>()?;
            let cc = u8::try_from(control.param).ok()?;
            if cc == 120 || cc == 123 {
                Some(Event::AllNotesOff)
            } else if cc == 7 {
                Some(Event::SetVolume {
                    value: Normalized::new((control.value.clamp(0, 127) as f32) / 127.0)
                        .expect("bounded controller value"),
                })
            } else {
                MacroId::from_cc(cc).map(|id| Event::SetMacro {
                    id,
                    value: Normalized::new((control.value.clamp(0, 127) as f32) / 127.0)
                        .expect("bounded controller value"),
                })
            }
        }
        EventType::Reset => Some(Event::AllNotesOff),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alsa::seq::{EvCtrl, EvNote};

    #[test]
    fn translates_notes_zero_velocity_macros_and_panic() {
        let note = EvNote {
            note: 64,
            velocity: 100,
            ..EvNote::default()
        };
        assert!(matches!(
            translate(&AlsaEvent::new(EventType::Noteon, &note)),
            Some(Event::NoteOn { note: 64, .. })
        ));
        let zero = EvNote {
            velocity: 0,
            ..note
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Noteon, &zero)),
            Some(Event::NoteOff { note: 64 })
        );
        let macro_cc = EvCtrl {
            param: 20,
            value: 127,
            ..EvCtrl::default()
        };
        assert!(matches!(
            translate(&AlsaEvent::new(EventType::Controller, &macro_cc)),
            Some(Event::SetMacro {
                id: MacroId::Evolve,
                ..
            })
        ));
        let volume = EvCtrl {
            param: 7,
            value: 63,
            ..EvCtrl::default()
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Controller, &volume)),
            Some(Event::SetVolume {
                value: Normalized::new(63.0 / 127.0).unwrap()
            })
        );
        let panic = EvCtrl {
            param: 123,
            ..EvCtrl::default()
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Controller, &panic)),
            Some(Event::AllNotesOff)
        );
    }
}
