use crate::control::{MacroId, Normalized};
use crate::dual_filter::{CORE_STATE_CC, CORE_TOGGLE_CC, DualFilterCore};
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
        EventType::Pitchbend => {
            let control = event.get_data::<EvCtrl>()?;
            Some(Event::PitchBend {
                value: control.value.clamp(-8192, 8191) as i16,
            })
        }
        EventType::Controller => {
            let control = event.get_data::<EvCtrl>()?;
            let cc = u8::try_from(control.param).ok()?;
            if cc == 120 || cc == 123 {
                Some(Event::AllNotesOff)
            } else if cc == 1 {
                Some(Event::Modulation {
                    value: control.value.clamp(0, 127) as u8,
                })
            } else if cc == 121 {
                Some(Event::ResetControllers)
            } else if cc == 7 {
                Some(Event::SetVolume {
                    value: Normalized::new((control.value.clamp(0, 127) as f32) / 127.0)
                        .expect("bounded controller value"),
                })
            } else if cc == CORE_TOGGLE_CC {
                (control.value > 0).then_some(Event::ToggleDualFilterCore)
            } else if cc == CORE_STATE_CC {
                Some(Event::SetDualFilterCore {
                    core: if control.value >= 64 {
                        DualFilterCore::Counter
                    } else {
                        DualFilterCore::Industrial
                    },
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
    fn translates_performance_wheels_and_controller_reset() {
        for value in [-8192, 0, 8191] {
            let control = EvCtrl {
                value,
                ..EvCtrl::default()
            };
            assert_eq!(
                translate(&AlsaEvent::new(EventType::Pitchbend, &control)),
                Some(Event::PitchBend {
                    value: value as i16
                })
            );
        }
        for value in [0, 64, 127] {
            let control = EvCtrl {
                param: 1,
                value,
                ..EvCtrl::default()
            };
            assert_eq!(
                translate(&AlsaEvent::new(EventType::Controller, &control)),
                Some(Event::Modulation { value: value as u8 })
            );
        }
        let reset = EvCtrl {
            param: 121,
            ..EvCtrl::default()
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Controller, &reset)),
            Some(Event::ResetControllers)
        );
    }

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
        let core_click = EvCtrl {
            param: u32::from(CORE_TOGGLE_CC),
            value: 127,
            ..EvCtrl::default()
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Controller, &core_click)),
            Some(Event::ToggleDualFilterCore)
        );
        let core_state = EvCtrl {
            param: u32::from(CORE_STATE_CC),
            value: 127,
            ..EvCtrl::default()
        };
        assert_eq!(
            translate(&AlsaEvent::new(EventType::Controller, &core_state)),
            Some(Event::SetDualFilterCore {
                core: DualFilterCore::Counter
            })
        );
    }
}
