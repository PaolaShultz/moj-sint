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

The current scalar voice combines the evidence-selected generic integrated-
wavetable oscillator with one experimental per-voice parallel character layer.
Shared 2,049-point antiderivative tables represent a saw and square target; each
voice performs linear table lookup, one-sample differentiation, and phase-
increment normalization. The untouched `SHAPE`/`COLOR` result is the dry
anchor. A parallel copy passes through a four-pole 6 kHz low-pass, a bounded
odd cubic soft clip, linear-component subtraction, a 10 Hz DC blocker, and a
two-stage note-tracked high-pass before an explicitly bounded inverted return.
The high-pass cutoff is prepared at 2.5 times note frequency and capped at
6 kHz. Direct evaluation was selected over first-order ADAA and a two-times
research variant by the recorded eight-times-reference alias/error rule. No
sample path allocates or performs per-sample trigonometric setup.

The rejected 0/120/240-degree selector remains a tested DSP primitive and a
direct disposable lab generator, but it is no longer in the voice render path.

## Modules

- `control`: the thirteen stable performance identities, normalized values,
  perceptual ADSR time mapping, and smoothing primitive.
- `dsp`: finite guards, the retained reference sine, independently implemented
  PolyBLEP and integrated-wavetable candidates, the negative-result shared-
  phase selector, and the parallel character layer. Frequency changes prepare
  phase increments, rotations, and note-tracked filter coefficients outside the
  sample loop.
- `envelope`: validated, sample-rate-aware ADSR state machine.
- `preset`: strict, versioned `.mojsint` TOML parsing and validation.
- `engine`: timestamped note/macro events, fixed voice storage, voice stealing,
  10 ms `SHAPE`/`COLOR`/`EDGE`/`COUPLE` smoothing, bounded level compensation,
  and block rendering. `SHAPE` morphs corrected saw to square; `COLOR` moves
  from a note-tracked one-pole dark path to the direct bright path. As an
  unaccepted research mapping, `EDGE` controls character drive/intensity and
  `COUPLE` controls return interaction. `COUPLE=0` is sample-identical and
  independent of `EDGE`.
- `analysis`: non-real-time candidate comparison against finite band-limited
  Fourier references. Its `alias_error_db` is explicitly a conservative sum
  of alias energy and amplitude/phase deviation after DC removal and fitted
  gain, not a perceptual score.
- `offline`: deterministic note rendering and stereo float WAV output.
- `main`: non-real-time `validate` and `render` commands.

## Deliberate deferrals

The stable macro names exist. `SHAPE`/`COLOR` have measured routes whose human
listening gate remains open. The selector's `EDGE`/`COUPLE` mapping was rejected
and retired from the engine. Their new parallel-character mapping has automated
evidence and a nine-file listening gate, but is not accepted as useful or as a
factory sound until the user listens. The other five timbral macro routes, more
distinctive oscillator/output systems, modulation, full control-usefulness
thresholds, live JACK/ALSA adapters, factory presets, and Pi profiling each need
their own measured, test-first milestone. No SIMD or architecture-specific path
should precede profiling on the Pi.
