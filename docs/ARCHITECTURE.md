# Architecture

## Scope and invariants

Moj Sint is a headless external instrument, not a DAW, plugin, or synthv1
compatibility layer. SHR-DAW owns its process. MIDI enters through one ALSA
Sequencer input and stereo audio leaves through exactly two JACK ports.

The library is deliberately independent of JACK, ALSA, files, processes, and
wall-clock time:

```text
versioned preset -> validated engine state
ALSA adapter -> bounded TimedEvent block -> Engine -> caller-owned L/R buffers
                                          ^
offline note specification ---------------+
```

`Engine::render_block` uses preallocated voices, caller-owned output buffers,
and a borrowed event slice. A test using `assert_no_alloc` guards this boundary.
The current scalar reference voice is only a sine oscillator and ADSR. It is a
deterministic baseline, not the intended finished sound.

## Modules

- `control`: the thirteen stable performance identities, normalized values,
  perceptual ADSR time mapping, and smoothing primitive.
- `dsp`: finite guards and the reference quadrature-recurrence oscillator;
  frequency changes prepare rotation coefficients outside the sample loop.
- `envelope`: validated, sample-rate-aware ADSR state machine.
- `preset`: strict, versioned `.mojsint` TOML parsing and validation.
- `engine`: timestamped events, fixed voice storage, voice stealing, and block
  rendering.
- `offline`: deterministic note rendering and stereo float WAV output.
- `main`: non-real-time `validate` and `render` commands.

## Deliberate deferrals

The stable macro names exist, but the nine timbral macro routes are not yet
claimed to be useful. Exotic oscillators, modulation, control-usefulness
thresholds, live JACK/ALSA adapters, factory presets, and Pi profiling each need
their own measured, test-first milestone. No SIMD or architecture-specific
path should precede profiling on the Pi.
