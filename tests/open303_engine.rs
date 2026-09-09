#![cfg(feature = "open303")]
use moj_sint::{
    engine::{Engine, Event, TimedEvent},
    preset::{Preset, SynthesisModelId},
};
#[cfg(debug_assertions)]
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

#[test]
fn factory_open303_roundtrips_and_renders_without_allocation() {
    let names = [
        "25-open303-rubber-bass",
        "26-open303-accent-wire",
        "27-open303-hollow-slide",
        "28-open303-soft-pluck",
    ];
    for name in names {
        let text = std::fs::read_to_string(format!("presets/{name}.mojsint")).unwrap();
        let preset = Preset::parse(&text).unwrap();
        assert_eq!(preset.model, SynthesisModelId::Open303);
        assert_eq!(preset.voices, 1);
        assert_eq!(Preset::parse(&preset.to_toml().unwrap()).unwrap(), preset);
        let mut engine = Engine::new(48000.0, &preset).unwrap();
        let mut l = [0.0; 512];
        let mut r = l;
        let events = [
            TimedEvent::new(
                0,
                Event::NoteOn {
                    note: 36,
                    velocity: 0.9,
                },
            ),
            TimedEvent::new(
                100,
                Event::NoteOn {
                    note: 43,
                    velocity: 0.5,
                },
            ),
            TimedEvent::new(200, Event::NoteOff { note: 43 }),
        ];
        assert_no_alloc::assert_no_alloc(|| engine.render_block(&events, &mut l, &mut r).unwrap());
        assert_eq!(l, r);
        assert!(l.iter().all(|v| v.is_finite() && v.abs() <= 1.0));
        assert!(l.iter().any(|v| v.abs() > 1e-5));
        engine
            .render_block(&[TimedEvent::new(0, Event::AllNotesOff)], &mut l, &mut r)
            .unwrap();
        assert!(l.iter().all(|v| *v == 0.0));
        assert!(Preset::parse(&text.replace("voices = 1", "voices = 2")).is_err());
    }
}

#[test]
fn live_control_motion_and_all_keys_remain_finite_and_allocation_free() {
    use moj_sint::control::{MacroId, Normalized};
    let source = include_str!("../presets/25-open303-rubber-bass.mojsint");
    for rate in [44100.0, 48000.0, 96000.0] {
        for filter in ["tb303", "lowpass18"] {
            let preset = Preset::parse(&source.replace("tb303", filter)).unwrap();
            let mut engine = Engine::new(rate, &preset).unwrap();
            let mut l = [0.0; 256];
            let mut r = l;
            assert_no_alloc::assert_no_alloc(|| {
                for key in 0..128 {
                    let events = [
                        TimedEvent::new(
                            0,
                            Event::NoteOn {
                                note: key,
                                velocity: if key % 2 == 0 { 1.0 } else { 0.5 },
                            },
                        ),
                        TimedEvent::new(
                            1,
                            Event::SetMacro {
                                id: MacroId::ALL[key as usize % 12],
                                value: Normalized::new(if key % 2 == 0 { 1.0 } else { 0.0 })
                                    .unwrap(),
                            },
                        ),
                    ];
                    engine.render_block(&events, &mut l, &mut r).unwrap();
                    assert!(l.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
                }
                for key in 0..128 {
                    engine
                        .render_block(
                            &[TimedEvent::new(0, Event::NoteOff { note: key })],
                            &mut l,
                            &mut r,
                        )
                        .unwrap();
                }
                for _ in 0..2000 {
                    engine.render_block(&[], &mut l, &mut r).unwrap();
                }
                assert_eq!(engine.active_voice_count(), 0);
                assert!(l.iter().all(|x| *x == 0.0));
            });
        }
    }
}

#[test]
fn every_native_surface_control_changes_held_note_audio() {
    use moj_sint::control::{MacroId, Normalized};
    fn render(index: usize, change: bool) -> Vec<f32> {
        let preset =
            Preset::parse(include_str!("../presets/25-open303-rubber-bass.mojsint")).unwrap();
        let mut engine = Engine::new(48000.0, &preset).unwrap();
        let mut output = vec![0.0; 12000];
        let mut right = vec![0.0; 12000];
        let velocity = if matches!(index, 6 | 9 | 10) {
            1.0
        } else {
            0.5
        };
        engine
            .render_block(
                &[TimedEvent::new(0, Event::NoteOn { note: 36, velocity })],
                &mut output[..1000],
                &mut right[..1000],
            )
            .unwrap();
        let value = if change {
            1.0
        } else {
            preset.macros.get(MacroId::ALL[index]).get()
        };
        engine
            .render_block(
                &[
                    TimedEvent::new(
                        0,
                        Event::SetMacro {
                            id: MacroId::ALL[index],
                            value: Normalized::new(value).unwrap(),
                        },
                    ),
                    TimedEvent::new(2000, Event::NoteOn { note: 48, velocity }),
                ],
                &mut output[1000..],
                &mut right[1000..],
            )
            .unwrap();
        output
    }
    for index in [0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11] {
        let a = render(index, false);
        let b = render(index, true);
        let difference: f32 = a.iter().zip(b).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            difference > 0.001,
            "inactive Open303 control index {index}: {difference}"
        );
    }
}
