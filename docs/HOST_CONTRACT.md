# Live Host Contract

The live host is implemented by the `moj-sint` binary:

```sh
moj-sint --client-name shs-moj-sint --preset /path/to/file.mojsint
```

The existing `validate` and `render` subcommands remain available.

## Process and ports

Before joining audio, the process reads one regular preset file no larger than
1 MiB and strictly validates its versioned schema. It opens JACK with
`JACK_NO_START_SERVER`, uses the configured client name, and registers exactly
two audio outputs with stable short names `out_l` and `out_r`. It never starts,
restarts, reconfigures, or auto-connects JACK.

The same configured name is used for one discoverable ALSA Sequencer client
with one subscribable MIDI input port named `input`. Setup failures are printed
outside the callback and return non-zero. SIGINT, SIGTERM, ALSA failure, JACK
shutdown, or a callback fault cause bounded shutdown: the MIDI thread exits,
the JACK client deactivates, and owned resources close.

## MIDI and timing

The ALSA thread translates:

- Note On, including velocity-zero Note On as Note Off;
- Note Off;
- CC 20–34 as the 15-position superset (older models retain CC 20–31);
- CC 35 as the press-only Dual Filter core toggle and CC 36 as exact core state;
- CC 120, CC 123, and Sequencer Reset as immediate All Notes Off.

It writes fixed-size events into a 1,024-slot SPSC queue. A full queue drops the
new event, increments an atomic overflow counter, and never blocks. Queue and
per-period overflow totals are reported only by non-real-time threads.

The ALSA thread calls JACK's non-process-thread
`jack_frames_since_cycle_start` query, associates the resulting offset with the
next callback cycle, and queues that schedule. The callback clamps current
period offsets, applies late events at offset zero, retains one future event,
orders the bounded per-period batch by offset, and passes it to
`Engine::render_block`.

## Real-time boundary

The callback uses caller-owned JACK buffers, preallocated engine voices, a
fixed 256-event period array, and the lock-free queue. It performs no allocation
or free, locks, file I/O, logging, formatting, process work, waiting, or
trigonometric/exponential coefficient setup. Model D cutoff and filter
coefficients are prepared before activation; macro smoothing and runtime
mappings use bounded scalar arithmetic.

The adapter choice is the small dynamic `libjack.so.0` FFI boundary already
used by SHR-DAW, plus `alsa` 0.7.1 for Sequencer input. This avoids a link-time
JACK development dependency while keeping unsafe JACK ownership in one module.
`libasound2-dev` is required to build the ALSA dependency.

SHR-DAW readiness requires the unambiguous configured or uniquely prefixed
client plus exactly the configured `out_l` and `out_r` ports. The host owns no
graph connections.
