# Live Host Contract

This document fixes the boundary for the later live-host milestone. It does not
claim that the current binary already implements the live host.

## Process and ports

The provisional invocation is:

```sh
moj-sint --client-name shs-moj-sint --preset /path/to/file.mojsint
```

The process must:

- use the exact configured JACK client name;
- register exactly two JACK audio outputs with stable short names `out_l` and
  `out_r`;
- publish one stable, discoverable ALSA Sequencer MIDI input;
- never start, restart, or reconfigure JACK;
- handle SIGINT and SIGTERM gracefully; and
- return a non-zero status with a clear diagnostic if setup fails.

SHR-DAW readiness is the unambiguous configured/prefixed JACK client plus
exactly two outputs. A MIDI port alone is not readiness.

## Thread boundary

The MIDI/control thread converts ALSA events into a fixed-capacity,
single-producer/single-consumer queue. Overflow has a counted, observable policy
and never blocks. The JACK callback drains only events for its current period,
maps timestamps to sample offsets, and calls `Engine::render_block`.

The callback must not allocate or deallocate, lock, access files, spawn
processes, log, format strings, panic, wait, or perform per-sample trigonometric
setup. Construction, preset parsing, port registration, and diagnostics happen
outside it. JACK's own API states that callback code must be suitable for
real-time execution: <https://jackaudio.org/api/group__NonCallbackAPI.html>.

## Adapter decision to make later

Evaluate both of these with a minimal prototype:

1. `jack` crate 0.13.x with default dynamic loading. It avoids a link-time JACK
   dependency and includes a controller/ring-buffer option, but the callback and
   shutdown behavior must be audited.
2. A small, auditable dynamic FFI layer matching SHR-DAW's `libjack.so.0`
   approach. This minimizes abstraction but increases local unsafe code.

For ALSA Sequencer, compare the maintained `alsa` crate's `seq` module with a
small FFI boundary. Either path requires `libasound2-dev` to build on Debian
systems. The GObject-based `alsaseq` crate adds GLib/event-loop machinery that
is unnecessary for this host.

## Future SHR-DAW work

Verified against `PaolaShultz/shr-daw` main commit
`8b7d0d7c17c582292ac06a915ca1fe750d77bc40` on 2026-07-22. A separate SHR-DAW
change must add:

- a fourth `BackendKind::MojSint` and exhaustive-match handling;
- a distinct Moj Sint `PresetId` and `.mojsint` catalog root/discovery rules;
- `moj_sint.command`, client, preset-root, MIDI destination, and port config;
- process argument construction and owned-process shutdown behavior;
- Project/Idea route identity and persistence;
- a Moj Sint CC schema for the twelve absolute controls, with pickup/reset;
- Playback master-encoder `EVOLVE` accumulation/reset that yields to N00B scale
  selection without losing the stored value; and
- backend-specific labels/help/status without altering synthv1 indices.

That change must retain SHR's All Notes Off, child-only termination, direct
playback transaction, optional owned graph, and unrelated-route protections.
