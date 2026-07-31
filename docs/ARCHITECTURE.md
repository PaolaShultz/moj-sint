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

The production voice is the circuit-informed Model D path: three independently
phased bandlimited VCOs, a continuously variable linear/nonlinear mixer,
prepared four-times-oversampled ladder, filter contour, output feedback, and a
separate live ADSR. Historical generic oscillators, character layers, and
listening labs remain research evidence but are not reached by `Engine`.
The production render is dual-mono; its two host ports do not claim stereo
generation.

The clean-room-oriented six-operator PM implementation is another isolated
research branch. It was independently authored from the cited mathematical
and functional facts; no expressive manual content, code, diagrams, presets,
voice data, or SysEx data was reproduced in the recorded process. Separate
legal review governs distribution. `dsp::six_op_pm` owns the fixed-capacity
graph and prepared sample state; `six_op_pm` owns only its listening inventory,
fixed event scheduler, renderer, and measurements. Neither module is reachable
from `Engine`, preset schemas, production controls, `host`, JACK, ALSA, or
SHR-DAW.

## Production control and polyphony contract

SHR-DAW exposes exactly twelve continuous synth controls. Moj Sint implements
eight Model D roles—`EVOLVE`, `SHAPE`, `COLOR`, `EDGE`, `COUPLE`, `MOTION`,
`DEPTH`, and `SPACE`—plus `ATTACK`, `DECAY`, `SUSTAIN`, and `RELEASE`.
`WIDTH` was removed because this Model D engine has no supported width
experiment. There is no hidden page or master-encoder takeover.

The catalog preserves all seven authored Model D audition starting points:
three full bass/lead/filter-articulation patches and four matched bass
diagnostics. They are selectable states of one modeled instrument, not separate
synthesis families. The eight timbral controls expose oscillator
character/drift, source balance, cutoff, mixer nonlinearity, feedback, filter
motion, ladder nonlinearity, and resonance from every start. EVOLVE uses 0 for
idealized oscillators, 0.5 for authored static character without drift, and 1
for full authored character, so the no-drift diagnostic is visible rather than
hidden preset state.

Parameter timing follows SHR-DAW's actual live-control path. Safe continuous
changes should reach held voices through prepared, smoothed events. A
particular structural or coefficient-heavy parameter may be applied only to
the next note when evidence shows that is necessary, but next-note-only
behavior is not a global engine constraint.

Production voice count is also unresolved. Native Raspberry Pi tests must
measure 1/2/4/8 voices in the real SHR/JACK workload. Four voices is a valid
possible product result; eight has no special status and is not required.

## Modules

- `control`: twelve stable identities, CC 20–31, normalized values,
  perceptual ADSR time mapping, and the smoothing primitive.
- `dsp`: finite guards, the retained reference sine, independently implemented
  PolyBLEP and integrated-wavetable candidates, the negative-result shared-
  phase selector, and the parallel character layer. Frequency changes prepare
  phase increments, rotations, and note-tracked filter coefficients outside the
  sample loop.
- `dsp::six_op_pm`: independently authored, fixed-capacity six-operator PM
  graph preparation and voice state. It validates 32 source-numbered
  connectivity shapes and carrier masks, prepares graph order, shared sine
  table, envelopes, operator increments/scaling, pitch envelope, and LFO, then
  renders through an allocation-free sample path. Declared feedback always
  uses the source operator's previous-sample output; this includes source
  algorithm 4's group feedback edge from operator 4 to operator 6.
- `envelope`: validated, sample-rate-aware ADSR state machine.
- `preset`: strict, versioned `.mojsint` TOML parsing and validation.
- `engine`: timestamped note/macro/panic events, fixed voice storage, voice
  stealing, 10 ms smoothing for every control, live ADSR, Model D macro
  mapping, finite guards, and dual-mono block rendering.
- `host`: dynamic JACK ownership/callback, ALSA Sequencer translation,
  fixed-capacity SPSC handoff, period timing, overflow reporting, and shutdown.
- `native_bench`: reusable native callback-simulation cases for the exact
  allocation-free `Engine::render_block` boundary and the separate isolated
  Model-D idealized research path. It prepares voices, buffers, and bounded
  events before timing; it is neither a JACK host nor production integration.
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
- `six_op_pm`: the isolated six-patch listening inventory, fixed four-slot
  event scheduler, dry dual-mono renderer, and non-real-time `measurements`
  submodule. The gate uses only source algorithms 1, 5, and 10, one shared
  score per pair, and fixed pair gains `0.25`, `0.23`, and `0.20`; it does not
  expose the 32-shape catalog as 32 listening variations.
- `dsp::hybrid`: three isolated, fixed-state, allocation-free complete voice
  topologies. Each combines multiple source, nonlinear, resonant, register,
  and spatial mechanisms and emits genuine stereo. These voices are not
  reachable from `Engine`, presets, or stable macros.
- `hybrid`: deterministic one-note, three-note chord, four-chord progression,
  and explicit mono-fold scheduling plus impact, harmony, stereo, and
  conservative high-rate residual measurements.
- `composite_machine`: isolated offline-only preparation and fixed-sample
  mixing of heterogeneous hybrid layers. It owns sample-accurate starts,
  prepared gains, stereo matrices, mute probes, a bounded cross-family
  follower, and one explicitly measured nonlinear junction. Its audition
  extension preserves raw renders, applies declared fixed hot-output gains,
  renders the same candidate mechanisms at D1/D2/D3 with one gain per
  topology, adds explicit D1 pedal sources, and owns a sample-accurate
  fixed-state research ADSR before source summing. It is not reachable from
  `Engine`, presets, or stable macros.
- `compact_composite`: the successor isolated listening boundary. Ten
  hardwired machines each own one to three prepared source mechanisms, a
  sample-accurate fixed score/envelope, fixed source/stereo gains, prepared
  pitch-drop state where applicable, 12 Hz pre/post-drive DC control, and an
  explicit linear, cubic, rational, or hard-clipping output policy. Tables and
  delay memory are allocated only during construction. The sample method is
  deterministic, finite, bounded to 0.999, and allocation-free. It is not
  reachable from `Engine`, presets, or stable macros.
- `main`: live host invocation plus non-real-time `validate` and `render`
  commands.

The separate `five-family-lab` binary renders those research sources into an
explicit output directory. Its mechanisms are nonlinear PM, excited comb,
four-path spatial micro-delay, authored additive spectral traversal, and a
24-machine fixed-width integer swarm. Only the micro-delay representative
generates distinct left and right signals; the other four are copied to both
channels. Construction prepares phase rotations, filter coefficients, delay
bounds, and fixed state; every sample path is allocation-free and bounded. The
binary performs file I/O and timing outside the sample path and is not a live
host.

The separate `six-op-pm-lab` binary is the only six-operator file-I/O and
workstation-timing boundary. It validates the complete gate before publishing
six 48 kHz, 32-bit-float dry dual-mono WAVs plus deterministic reports. It
refuses an existing non-empty destination and applies no post-render clipper,
normalization, effects, limiting, filtering, or mastering. `SixOpVoice` retains
an emergency safety clamp; accepted renders recorded zero contacts. The lab's
scalar offline timing is not live-callback, Raspberry Pi, latency, polyphony,
or sound-quality evidence.

The separate `hybrid-sound-lab` binary renders twelve complete-voice files:
three structurally different families crossed with single-note, held-chord,
progression, and progression-mono conditions. The families are a
cross-coupled machine voice, an authored spectral-shadow voice, and a
noise/register-excited dual-resonant body. Integer state is one composable
mechanism inside each topology rather than an all-integer instrument boundary.
All nonlinear voice processing occurs before the linear three-note mix so the
chord report can distinguish per-note identity from post-mix intermodulation.

The separate `composite-machine-lab` binary retains the old reconstruction
code and exact hashes but now writes a compact power listening batch. The hot
delayed estimate is one orientation reference; ten primary files then cover
sustained low, playable mid, evolving, pedal, bass, two thumps, kick, struck,
and musical-context roles. Six bass-bearing mono folds come last. Every
primary uses at most three simultaneous mechanisms and one declared gain/drive
policy across D1/D2/D3 and bounded tone tests. There is no analysis-dependent
limiter or per-file normalization. Explicit saturation, hard clipping, and the
0.999 digital ceiling are part of named topologies. Full evidence and
limitations are in `COMPOSITE_MACHINE_RESEARCH.md`.

## Deliberate deferrals

The provisional macro candidates exist, but the final eight timbral names
remain open. `SHAPE`/`COLOR` have measured routes whose human listening gate
remains open. The selector's `EDGE`/`COUPLE` mapping was rejected and retired
from the engine. The replacement parallel-character mapping also failed human
listening because it presented the same weak source under small treatments; it
is not an accepted factory sound or mapping. The completed
five-family research gate remains isolated from `Engine` and selected no
successor. The complete hybrid stereo/chord gate is now implemented and also
remains isolated pending human headphone, speaker, and mono listening. Its
automated bounds do not select a family or establish a useful stable route.
The first positive combined listening accident now has a compact successor
batch with ten one-to-three-mechanism hypotheses. Automated loudness-floor,
tone/pitch retention, ablation, event, mono, DC, ceiling, residual, and
determinism checks pass, but none establishes perceived power, distinction, or
musical value. Digital level is not acoustic SPL.
The clean-room-oriented six-operator PM gate now passes its finite, level, DC,
jump, pair-RMS, pitch, spectral, alias/error, sweep, and determinism bounds. It
still selects no graph, patch, preset, or control mapping: human listening of
the three exact reference/original pairs is the next research acceptance gate.
The future typed micro-machine graph is specified in
`MICRO_MACHINE_ROUTING.md`, but extraction starts only after listening reveals
which nodes and connections deserve reuse. Selection and mapping of the final
eight timbral roles, modulation, full control-usefulness thresholds, live
JACK/ALSA adapters, factory presets, and Pi profiling each need their own
measured, test-first milestone. No SIMD or architecture-specific path should
precede profiling on the Pi.
