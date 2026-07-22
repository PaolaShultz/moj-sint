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
and a borrowed event slice. Tests using `assert_no_alloc` guard both candidate
oscillator sample paths and the complete block boundary, including rapid macro
events.

The current scalar voice uses the evidence-selected generic integrated-
wavetable oscillator. Shared 2,049-point antiderivative tables represent a saw
and square target; each voice performs linear table lookup, one-sample
differentiation, and phase-increment normalization. Tables are immutable and
shared rather than copied per voice.

## Modules

- `control`: the thirteen stable performance identities, normalized values,
  perceptual ADSR time mapping, and smoothing primitive.
- `dsp`: finite guards, the retained reference sine, and independently
  implemented PolyBLEP and integrated-wavetable candidates. Frequency changes
  prepare phase increments outside the sample loop.
- `envelope`: validated, sample-rate-aware ADSR state machine.
- `preset`: strict, versioned `.mojsint` TOML parsing and validation.
- `engine`: timestamped note/macro events, fixed voice storage, voice stealing,
  10 ms `SHAPE`/`COLOR` smoothing, bounded level compensation, and block
  rendering. `SHAPE` morphs corrected saw to square; `COLOR` moves from a
  note-tracked one-pole dark path to the direct bright path.
- `analysis`: non-real-time candidate comparison against finite band-limited
  Fourier references. Its `alias_error_db` is explicitly a conservative sum
  of alias energy and amplitude/phase deviation after DC removal and fitted
  gain, not a perceptual score.
- `offline`: deterministic note rendering and stereo float WAV output.
- `main`: non-real-time `validate` and `render` commands.

## Deliberate deferrals

The stable macro names exist, and `SHAPE`/`COLOR` now have measured routes, but
human listening acceptance remains open. The other seven timbral macro routes,
more exotic oscillators, modulation, full control-usefulness thresholds, live
JACK/ALSA adapters, factory presets, and Pi profiling each need their own
measured, test-first milestone. No SIMD or architecture-specific path should
precede profiling on the Pi.
