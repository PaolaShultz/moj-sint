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
events. Model D and Six-Op PM remain dual-mono; Strange Oscillator preserves
its model-owned stereo through the same two output buffers.

The host/model/instrument boundary is explicit: the `moj-sint` process is the
engine, each preset selects a `SynthesisModelId`, and the preset is the
instrument. Each voice dispatches through a fixed `VoiceModel` enum prepared
before the callback; models do not share DSP state or add runtime allocation.

The first live model is the circuit-informed Model D path: three independently
phased bandlimited VCOs, a continuously variable linear/nonlinear mixer,
prepared four-times-oversampled ladder, filter contour, output feedback, and a
separate live ADSR. Historical generic oscillators, character layers, and
listening labs remain research evidence but are not reached by `Engine`.
The live render is dual-mono; its two host ports do not claim stereo
generation.

The second live model is Six-Op PM. Its six independently authored
listening patches feed a small `six_op_pm::live` adapter that retains one
prepared sine table, selects a fixed graph and patch before rendering, and
exposes the same preallocated `VoiceModel` boundary as Model D. The historical
lab, scheduler, renderers, and measurement reports remain research tools; they
are not a parallel live engine. Separate legal review still governs
distribution of the clean-room-oriented implementation.

The third live model is Strange Oscillator. One `TYPE` macro selects
eight source topologies through a prepared 10 ms crossfade. Seven shared
structural controls then expose large form, deformation, coupling, motion,
irregularity, color, and stereo changes. Its public start is an experimental
instrument whose musical verdict and native Raspberry Pi headroom remain open.

Swarm Machine is the fourth live model. It compiles the strict schema-1 typed
graph before voice construction and renders the prepared nine-oscillator warm
pad through fixed state. Bass Matrix is the fifth: a phase-locked sub/body,
separate upper character paths, selective nonlinear processing, filtering,
feedback, DC removal, and an output guard. Dual Filter is the sixth. It owns
two independently enveloped filter blocks, continuous serial/parallel
structure, and a click-selected INDUSTRIAL or COUNTER core. These models use
the same preallocated `VoiceModel` and stereo host boundary as the earlier
three.

## Live control and polyphony contract

The first five models retain twelve normalized timbre/ADSR values. Model D
implements `EVOLVE`, `SHAPE`, `COLOR`, `EDGE`, `COUPLE`, `MOTION`, `DEPTH`,
and `SPACE`. Six-Op PM implements `INDEX`, `RATIO`, `FEEDBACK`, `OP DECAY`,
`BALANCE`, `KEY SCALE`, `VELOCITY`, and `MOTION`. Strange Oscillator implements
`TYPE`, `FORM`, `WARP`, `COUPLE`, `MOTION`, `CHAOS`, `COLOR`, and `SPACE`.
Those models append `ATTACK`, `DECAY`, `SUSTAIN`, and `RELEASE`. In SHR's
physical surface, position 5 instead controls independent instrument volume
(CC7); the historical fifth timbre macro remains in the schema and MIDI
interface for compatibility. Thus these models expose seven timbre controls,
volume, and ADSR on twelve physical positions. Pressure Chain retains all
eight timbre controls plus amp ADSR, including SWEEP at position 5. See the
[preset and control schema](PRESET_SCHEMA.md) for the exact mappings. SHR owns
positions 13–15 as Project AUX sends for these twelve-position models.

Dual Filter uses 15 positions: two cutoff/resonance/envelope-depth blocks, STRUCTURE,
filter ADSR, and amp ADSR. Its dedicated synth click toggles only the backstage
INDUSTRIAL/COUNTER core; the master encoder remains SHR navigation.
`WIDTH` was removed because this Model D engine has no supported width
experiment. There is no hidden page or master-encoder takeover.

The catalog preserves seven authored Model D audition starting points, six
accepted Six-Op PM listening starts, one Strange Oscillator start, one Swarm
Machine start, one Bass Matrix start, five Dual Filter starts, and three
Pressure Chain starts, and four Open303 starts: 28 cleared factory presets in total.
Model D contains three full bass/lead/filter-articulation patches and four matched bass
diagnostics. They are selectable states of one modeled instrument, not separate
synthesis families. The eight engine timbral values describe oscillator
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

Whole-system voice budgets for polyphonic models remain unresolved. Native
Raspberry Pi tests must measure 1/2/4/8 voices in the real SHR/JACK workload;
earlier foundation-engine simulations do not establish every model's budget.
Four voices is a valid possible result; eight is not required. Pressure Chain
is explicitly monophonic and rejects presets requesting more than one voice.

## Modules

- `control`: the stable CC 20–34 positional superset, normalized values,
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
  stealing, 10 ms smoothing for every control, live ADSR, model dispatch,
  finite guards, and bounded stereo block rendering.
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
- `offline`: deterministic two-channel float WAV output that retains each
  model's dual-mono or stereo result.
- `micro_machine`: strict schema-1 TOML parsing, typed-port
  validation, deterministic topological compilation, fixed resource
  accounting, the seven-node swarm vocabulary, its eight timbral controls,
  and an outer-ADSR experimental voice. Its compiled plan and fixed state are
  allocation-free in sample/block rendering. The authored swarm graph is
  compiled while voices are constructed and is reachable through the live
  `swarm_machine` model; parsing and compilation never occur in the callback.
- `bass_matrix`: fixed-state bass voice with a phase-locked half-frequency
  body, PM/growl and inharmonic upper branches, selective two-substep nonlinear
  processing, note-tracked filtering, bounded feedback, DC removal, and a
  static output guard. The linear body stays outside the driven branch.
- `micro_machine_lab`: non-real-time MIDI-note conversion and conservative
  48 kHz versus 384 kHz fitted residual measurement for that experiment.
- `dsp::research`: five disposable, monophonic, allocation-free scalar source
  states used only by the five-family listening gate. They are not variants of
  the production voice and are not reachable from `Engine` or presets.
- `research`: non-real-time rendering, loudness matching, spectral and spatial
  measurements, conservative high-rate residual comparison, and deterministic
  hashing for the disposable sources.
- `six_op_pm`: the six-patch inventory, production `live` adapter, fixed
  four-slot research scheduler, dry dual-mono renderer, and non-real-time
  `measurements` submodule. The selected starts use source algorithms 1, 5,
  and 10; the live adapter does not expose the 32-shape catalog as 32 choices.
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

The separate `micro-machine-lab` binary is the swarm graph's offline file-I/O
boundary. It compiles one explicit graph and writes a neutral reference, lead,
pad, eight control demonstrations, and dry resource/level/stereo/mono/error
reports into an empty output directory. It does not run in the live callback;
the same compiled graph now supplies the Swarm Machine production voice.

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

The Model D `SHAPE`/`COLOR` routes still have an open human listening gate.
The selector's former `EDGE`/`COUPLE` mapping was rejected and retired
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
The clean-room-oriented six-operator PM gate passed its finite, level, DC,
jump, pair-RMS, pitch, spectral, alias/error, sweep, and determinism bounds.
Its six authored sounds are now selectable live starts under one Six-Op
PM model with eight bounded timbral controls. Musical acceptance in the live
SHR/JACK path and native Pi headroom remain open; automated evidence does not
claim either.
The first typed micro-machine slice is specified in
`MICRO_MACHINE_ROUTING.md`. It compiles one swarm-oriented TOML vocabulary and
is now the fourth live model, without approving runtime graph editing or a
general graph runtime. Musical acceptance and native Pi profiling remain open.
No SIMD or architecture-specific path should precede profiling on the Pi.

## Pressure Chain live boundary

The seventh model wraps the original `pressure_chain::PressureChainVoice`
with a fixed 128-key last-note stack. One preset selects one of three
preallocated topologies and must declare exactly one voice. Its amp ADSR and
filter contour stay inside the renderer; Engine adds only the existing output
gain and instrument-volume smoothing. Coefficient tables are prepared before
rendering, and note/control/render/recovery operations allocate no storage.
The callback does not parse presets or choose/create new synth processes.

Open303 is a monophonic native voice behind the reviewed C ABI. Its allocation
and table preparation happen before rendering. Engine smoothing feeds cached
control setup every 32 samples, and exact note offsets use the native fixed
note stack. It owns its envelope interaction; the generic ADSR is bypassed.
See [Open303 integration](OPEN303_INTEGRATION.md) for the explicit control map.
