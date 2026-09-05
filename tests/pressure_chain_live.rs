use assert_no_alloc::assert_no_alloc;
use moj_sint::{
    engine::{Engine, Event, TimedEvent},
    preset::Preset,
};
#[global_allocator]
static ALLOC: assert_no_alloc::AllocDisabler = assert_no_alloc::AllocDisabler;

const START: &str = include_str!("../presets/22-pressure-chain-deep-cascade.mojsint");
#[test]
fn pressure_chain_schema_round_trip_and_mono_boundary() {
    let p = Preset::parse(START).unwrap();
    assert_eq!(p, Preset::parse(&p.to_toml().unwrap()).unwrap());
    assert!(Preset::parse(&START.replace("voices = 1", "voices = 2")).is_err());
    assert!(Preset::parse(&START.replace("schema_version = 9", "schema_version = 8")).is_err());
    assert!(Preset::parse(&START.replace("bite = 0.42", "bite = 2.0")).is_err());
}
#[test]
fn pressure_chain_live_notes_controls_panic_are_bounded_and_allocation_free() {
    let p = Preset::parse(START).unwrap();
    let mut engine = Engine::new(48_000.0, &p).unwrap();
    let mut l = [0.0; 4096];
    let mut r = [0.0; 4096];
    assert_no_alloc(|| {
        engine
            .render_block(
                &[
                    TimedEvent::new(
                        0,
                        Event::NoteOn {
                            note: 36,
                            velocity: 0.9,
                        },
                    ),
                    TimedEvent::new(
                        1024,
                        Event::NoteOn {
                            note: 48,
                            velocity: 0.8,
                        },
                    ),
                    TimedEvent::new(2048, Event::NoteOff { note: 48 }),
                    TimedEvent::new(3072, Event::AllNotesOff),
                ],
                &mut l,
                &mut r,
            )
            .unwrap();
    });
    assert_eq!(l, r);
    assert!(l[..3072].iter().any(|s| s.abs() > 0.01));
    assert!(l.iter().all(|s| s.is_finite() && s.abs() <= 0.94));
    assert!(l[3072..].iter().all(|s| *s == 0.0));
    assert_eq!(engine.active_voice_count(), 0);
}

#[test]
fn all_factory_pressure_topologies_are_distinct_and_reproducible() {
    let sources = [
        START,
        include_str!("../presets/23-pressure-chain-body-tap.mojsint"),
        include_str!("../presets/24-pressure-chain-cross-feed.mojsint"),
    ];
    let mut outputs = Vec::new();
    for source in sources {
        let preset = Preset::parse(source).unwrap();
        assert_eq!(preset, Preset::parse(&preset.to_toml().unwrap()).unwrap());
        let mut previous = None;
        for _ in 0..2 {
            let mut engine = Engine::new(48_000.0, &preset).unwrap();
            let mut left = vec![0.0; 4096];
            let mut right = vec![0.0; 4096];
            engine
                .render_block(
                    &[TimedEvent::new(
                        0,
                        Event::NoteOn {
                            note: 40,
                            velocity: 0.85,
                        },
                    )],
                    &mut left,
                    &mut right,
                )
                .unwrap();
            if let Some(old) = previous {
                assert_eq!(left, old);
            }
            previous = Some(left);
        }
        outputs.push(previous.unwrap());
    }
    for a in 0..3 {
        for b in a + 1..3 {
            assert_ne!(outputs[a], outputs[b]);
        }
    }
}

#[test]
fn held_note_return_slides_and_stale_releases_do_not_cut_the_new_note() {
    let preset = Preset::parse(START).unwrap();
    let mut engine = Engine::new(48_000.0, &preset).unwrap();
    let mut l = [0.0; 1024];
    let mut r = [0.0; 1024];
    let blocks = [
        Event::NoteOn {
            note: 36,
            velocity: 0.9,
        },
        Event::NoteOn {
            note: 48,
            velocity: 0.8,
        },
        Event::NoteOff { note: 48 },
        Event::NoteOff { note: 48 },
    ];
    for event in blocks {
        engine
            .render_block(&[TimedEvent::new(0, event)], &mut l, &mut r)
            .unwrap();
        assert_eq!(engine.active_voice_count(), 1);
        assert!(l.iter().any(|s| s.abs() > 0.01));
    }
    engine
        .render_block(
            &[TimedEvent::new(0, Event::NoteOff { note: 36 })],
            &mut l,
            &mut r,
        )
        .unwrap();
    for _ in 0..20 {
        engine.render_block(&[], &mut l, &mut r).unwrap();
    }
    assert_eq!(engine.active_voice_count(), 0);
    assert_eq!(l, [0.0; 1024]);
}

#[test]
fn full_key_stack_and_live_control_changes_do_not_allocate_or_overflow() {
    use moj_sint::control::{MacroId, Normalized};
    let preset = Preset::parse(START).unwrap();
    let mut engine = Engine::new(48_000.0, &preset).unwrap();
    let mut l = [0.0; 256];
    let mut r = [0.0; 256];
    assert_no_alloc(|| {
        for note in 0..=127 {
            engine
                .render_block(
                    &[TimedEvent::new(
                        0,
                        Event::NoteOn {
                            note,
                            velocity: 1.0,
                        },
                    )],
                    &mut l,
                    &mut r,
                )
                .unwrap();
        }
        engine
            .render_block(
                &[TimedEvent::new(
                    0,
                    Event::NoteOn {
                        note: 127,
                        velocity: 1.0,
                    },
                )],
                &mut l,
                &mut r,
            )
            .unwrap();
        for id in MacroId::ALL.into_iter().take(12) {
            for value in [0.0, 1.0] {
                engine
                    .render_block(
                        &[TimedEvent::new(
                            0,
                            Event::SetMacro {
                                id,
                                value: Normalized::new(value).unwrap(),
                            },
                        )],
                        &mut l,
                        &mut r,
                    )
                    .unwrap();
                assert!(l.iter().all(|s| s.is_finite() && s.abs() <= 0.94));
                assert!(l.windows(2).all(|s| (s[1] - s[0]).abs() < 0.9));
            }
        }
        engine
            .render_block(&[TimedEvent::new(0, Event::AllNotesOff)], &mut l, &mut r)
            .unwrap();
    });
    assert_eq!(l, [0.0; 256]);
    engine
        .render_block(
            &[
                TimedEvent::new(
                    0,
                    Event::NoteOn {
                        note: 36,
                        velocity: 0.9,
                    },
                ),
                TimedEvent::new(128, Event::NoteOff { note: 36 }),
            ],
            &mut l,
            &mut r,
        )
        .unwrap();
    // Restoring all old held notes would leave an active gate after release.
    for _ in 0..4000 {
        engine.render_block(&[], &mut l, &mut r).unwrap();
    }
    assert_eq!(engine.active_voice_count(), 0);
}
