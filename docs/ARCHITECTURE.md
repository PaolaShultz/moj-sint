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
events. The current production engine writes the same mono mix to both output
buffers; two JACK ports and two-channel WAV metadata do not yet imply stereo
sound generation.

The current scalar voice still contains the evidence-selected generic
integrated-wavetable oscillator and the rejected per-voice parallel character
layer. The latter remains implementation residue for reproducibility, not an
accepted factory sound or macro mapping.
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
- `offline`: deterministic two-channel float WAV output, currently dual-mono
  when rendering `Engine`.
- `dsp::research`: five disposable, monophonic, allocation-free scalar source
  states used only by the five-family listening gate. They are not variants of
  the production voice and are not reachable from `Engine` or presets.
- `research`: non-real-time rendering, loudness matching, spectral and spatial
  measurements, conservative high-rate residual comparison, and deterministic
  hashing for the disposable sources.
- `main`: non-real-time `validate` and `render` commands.

The separate `five-family-lab` binary renders those research sources into an
explicit output directory. Its mechanisms are nonlinear PM, excited comb,
four-path spatial micro-delay, authored additive spectral traversal, and a
24-machine fixed-width integer swarm. Only the micro-delay representative
generates distinct left and right signals; the other four are copied to both
channels. Construction prepares phase rotations, filter coefficients, delay
bounds, and fixed state; every sample path is allocation-free and bounded. The
binary performs file I/O and timing outside the sample path and is not a live
host.

## Deliberate deferrals

The stable macro names exist. `SHAPE`/`COLOR` have measured routes whose human
listening gate remains open. The selector's `EDGE`/`COUPLE` mapping was rejected
and retired from the engine. The replacement parallel-character mapping also
failed human listening because it presented the same weak source under small
treatments; it is not an accepted factory sound or mapping. The completed
five-family research gate remains isolated from `Engine` and selected no
successor. The next milestone is a second mono-first sound-identity gate with
deliberate stereo topology, not more output treatments of the current dry
source. The other five timbral macro routes, modulation, full
control-usefulness thresholds, live JACK/ALSA adapters, factory presets, and Pi
profiling each need their own measured, test-first milestone. No SIMD or
architecture-specific path should precede profiling on the Pi.
