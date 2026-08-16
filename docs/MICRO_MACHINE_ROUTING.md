# Typed Micro-Machine Routing Roadmap

Date: 2026-07-23

## Purpose

Moj Sint should eventually support rapid experiments built by connecting small,
heterogeneous DSP “micro-machines”: integer VCOs, carry-event sources, phase
delays, crest followers, shapers, resonators, filters, stereo matrices, and
other bounded state elements.

The goal is not a generic modular-synth clone. It is a machine-native
laboratory where unusual connection order and state exchange can be tried
quickly, measured reproducibly, and compiled into a safe fixed real-time graph.

Established models remain hardwired Rust signal paths. One deliberately narrow
vertical slice now exists as the live Swarm Machine described below. It proves
that one validated graph can be compiled during voice construction and rendered
through the production `Engine` without making arbitrary graph editing live or
creating a general modular framework. The broader musician and community intent is recorded in
[Experimental direction](FUTURE_DIRECTION.md).

## Example guidance, not an approved design

The user's illustrative idea was:

```text
base source -------------------------------> controlled mix
    |
    +-> absolute value -> envelope/crest follower
                            -> threshold with hysteresis -> trigger
                                                           |
base phase -> explicit phase delay -> integer VCO ----------+
                                                           |
                                              crest-shaping branch
```

The value of the example is the connection pattern: an integer oscillator can
be triggered or modulated by an audio-derived condition and can in turn shape a
different source. The particular threshold, phase delay, “superint VCO,” and
crest algorithm are not approved product behavior or a scheduled experiment.

## Typed ports

Free routing must not mean treating every value as an interchangeable `f32`.
Nodes expose explicit port domains:

- `AudioMono` and `AudioStereo`;
- normalized smoothed `Control`;
- `Phase` and `PhaseIncrement`;
- one-sample `Trigger`;
- bounded integer `Word<N>`;
- `Carry`, `Borrow`, or other sparse machine events;
- envelope/crest `Measurement`; and
- declared delayed feedback state.

The graph compiler rejects incompatible connections unless an explicit
conversion node exists. Examples include integer-to-normalized-audio,
audio-to-envelope, phase division, trigger-to-envelope, mono-to-stereo matrix,
and bounded event injection.

## Candidate node vocabulary

The first useful library is likely to include:

- float and fixed-width phase accumulators;
- wrapping register transitions, rotates, shifts, carry, and borrow;
- sine/table/phase-law oscillators;
- phase offsets, dividers, and prepared delays;
- absolute-value, RMS/envelope, transient, and crest followers;
- hysteretic comparators, refractory triggers, gates, and pulse smoothers;
- band splitters and note-tracked filters;
- bounded odd/asymmetric shapers and generated-component extraction;
- all-pass phase rotation and explicit DC blockers;
- combs, resonators, and delayed feedback junctions;
- mid/side, cross-coupling, micro-delay, and stereo matrix nodes;
- bounded mixers, gain compensation, and output guards.

Integer mechanisms remain composable. They may generate sound directly, but
they may also excite, address, perturb, divide, gate, couple, or measure another
mechanism. A bank of similar integer voices is one possible graph, not the
architectural default.

## Compile boundary

The editable experiment description is validated and compiled outside the
audio callback:

```text
versioned graph description
  -> parse and validate node/port types
  -> reject unbounded or instantaneous cycles
  -> prepare coefficients, tables, masks, and delays
  -> topologically schedule nodes
  -> allocate fixed node and edge storage
  -> produce immutable execution plan + mutable fixed state
  -> render caller-owned buffers
```

The callback never parses, allocates, deallocates, locks, performs I/O, logs,
formats, spawns, or discovers connections. Graph edits create a new compiled
instance outside real time and swap only through a separately designed bounded
handoff.

## Feedback and triggers

- A cycle is legal only through an explicit one-sample, block, comb, resonator,
  or other declared delay node.
- Every feedback edge declares gain/energy bounds and a reset policy.
- Nonlinear feedback requires finite-state, DC, energy-growth, decay, and
  alias/error evidence.
- Audio-derived thresholds use smoothing plus hysteresis or a refractory
  interval; raw `abs(sample) > threshold` edges are rejected because they can
  chatter at audio rate.
- Sparse integer events declare maximum impulse magnitude, minimum event
  spacing, and conversion/band-limiting behavior.

## Experiment format

A versioned TOML graph is the likely authoring surface because Moj Sint already
uses strict TOML presets and has no UI. The first graph format should remain a
research tool, not silently become preset version 2.

It must express:

- node identity and type;
- typed input/output connections;
- constructor parameters and legal ranges;
- deterministic seeds and phase policy;
- explicit feedback delays;
- mono/stereo output mapping; and
- named probes for diagnostic stems and measurements.

Unknown fields, unknown node types, incompatible ports, duplicate identifiers,
unbounded cycles, non-finite values, and resource-limit violations fail before
rendering.

## Implemented schema 1 swarm slice

The first tracked graph is
`experiments/swarm-micro-machine-v1.toml`. It is a research format, not a
`.mojsint` preset schema. Its top-level form is strict:

| Field | Meaning |
| --- | --- |
| `schema_version` | Exactly `1` |
| `name` | Non-empty experiment name |
| `seed` | Deterministic `u64` phase and machine-state seed |
| `controls` | Exactly `MASS`, `DETUNE`, `SPREAD`, `SHAPE`, `BITE`, `MOTION`, `COLOR`, `SPACE`, in that order |
| `output` | One `node.port` reference that resolves to `GuardedStereo` |
| `[[nodes]]` | Ordered node declarations with unique lowercase IDs |

Every node has `id`, `type`, and, except for the source, one `input` reference.
Constructor fields are legal only on their owning type; a known field on the
wrong node is rejected rather than ignored.

| Node type | Input | Output | Constructor fields |
| --- | --- | --- | --- |
| `phase_swarm` | none | `phases: PhaseBank` | `max_oscillators`, `detune_cents`, `drift_cents`, `coupling` |
| `bandlimited_saw_bank` | `PhaseBank` | `waves: OscillatorBank` | none |
| `bounded_stereo_mix` | `OscillatorBank` | `stereo: AudioStereo` | none |
| `spectral_tilt` | `AudioStereo` | `stereo: AudioStereo` | `min_cutoff_hz`, `max_cutoff_hz` |
| `bounded_drive` | `AudioStereo` | `stereo: AudioStereo` | `drive` |
| `stereo_width` | `AudioStereo` | `stereo: AudioStereo` | `max_width` |
| `output_guard` | `AudioStereo` | `output: GuardedStereo` | `ceiling` |

This vocabulary is intentionally unable to describe an arbitrary synth. It
separates phase population, waveform generation, normalization and placement,
spectral shaping, nonlinearity, stereo width, and guarding so the swarm is not
an opaque supersaw node. It has no feedback/delay node, so every cycle is an
invalid instantaneous cycle.

Compilation validates the complete graph, uses source order as the stable tie
breaker in a topological schedule, prepares all coefficients, phase offsets,
drift rotations, gear seeds, node state, and signal storage, and returns an
immutable boxed plan plus a matching fixed boxed state. The live path only
advances eight fixed control smoothers, executes that schedule, and writes a
frame or caller-owned block. It performs no graph edit or discovery.

Current compile limits are 16 nodes, 16 edges, 12 oscillator slots, 64 KiB of
node state, and 1,024 conservative work units per sample. The tracked swarm
uses seven nodes, six edges, nine oscillator slots, 3,416 state bytes, and 355
work units. These are portable scalar bounds, not measured timing or a safe Pi
voice count.

MASS continuously admits the preallocated phase population and the mixer
normalizes by active squared weight. DETUNE scales symmetric phase increments;
MOTION adds prepared low-rate drift plus deterministic wrap-driven gear
perturbation. SPREAD pans individual machines, SHAPE morphs the independently
bandlimited waveform bank, and BITE, COLOR, and SPACE belong to explicit
downstream nodes. ADSR remains outside the graph through the existing Moj Sint
envelope type.

`micro-machine-lab render <graph.toml> <empty-output-directory>` produces one
neutral reference, one lead, one pad, and eight low/high control reels plus
resource, level/stereo/mono, deterministic hash, and conservative high-rate
error reports. The dry outputs contain no production effects.

## Resource model

The compiler enforces fixed maxima for nodes, edges, state bytes, delay memory,
and per-sample work estimates. Scalar Rust remains the baseline. Workstation
timing helps compare graphs, but only native Raspberry Pi callback/headroom
evidence can establish production graph limits, latency, or safe polyphony.

## Evidence for every compiled graph

- parse/validation failures are deterministic and clear;
- identical graph/seed/input produces identical output;
- sample and block paths allocate nothing;
- all output and internal guarded states remain finite and bounded;
- cycles have measured energy growth/decay;
- integer periods, lockups, and sample-rate relationships are reported;
- oscillator/nonlinear nodes receive high-rate alias/error comparisons;
- triggers cannot chatter beyond their declared rate;
- chords receive per-note retention and intermodulation evidence;
- stereo graphs report actual L/R difference, mid/side by band, correlation,
  and mono fold-down;
- resource and scalar cost reports are explicit and reproducible.

Human listening remains the acceptance gate for musical usefulness.

## Development sequence

1. Finish the three hardwired complete hybrid voices and chord/stereo gate.
2. Record which mechanisms and connection patterns were actually audible and
   useful; remove inert or redundant layers.
3. Extract only those successful mechanisms into focused fixed-state nodes.
4. Implement a Rust builder and typed graph validator first. The narrow swarm
   slice now supplies this evidence without a universal builder API.
5. Add the versioned TOML experiment description after the validation rules
   are explicit. Schema 1 now covers only the swarm vocabulary above.
6. Build an offline graph lab with harmony renders, stereo evidence,
   alias/error measurement, resource reporting, and deterministic hashes. The
   swarm lab now supplies this first slice.
7. Run human listening gates on compiled experimental graphs. The owner selected
   the warm pad as the useful hypothesis, and it is now the one factory start.
8. Keep graph parsing/compilation outside the callback and profile the live
   fixed graph on Raspberry Pi before widening the accepted voice budget or
   graph vocabulary.

This sequence preserves the enormous routing possibility without committing
the live instrument to abstractions that have not yet produced worthwhile
sound.

## 2026-07-23 composite evidence

The composite-machine lab hardwired two typed connection patterns and found
them active under automated evidence:

```text
AudioStereo -> Mid -> EnvelopeMeasurement
EnvelopeMeasurement -> bounded ControlGain
EnvelopeMeasurement -> bounded StereoSideScale
```

The follower traversed 0.75-1.25 and mute tests show all three source,
destination, and side-response layers materially affect output.

The risky original candidate also tested:

```text
AudioMono x AudioMono -> ProductAudio
ProductAudio -> internal DC control -> bounded StereoMatrix
```

Its first revision was near-identical to another graph and its second produced
excess difference products. The retained hardwired revision has isolated
junction products 31.281 dB below the weakest scheduled target, but its
isolated 8x residual is still -3.670 dB. This is evidence that product
junctions require a distinct port/type, linear-control subtraction, DC state,
and explicit product/residual reports. It is not evidence that the node is
musically useful.

The scheduler also validated candidate concepts for a future score/event
boundary: deterministic seed/phase policy, sample-offset start, prepared gain,
2x2 stereo matrix, and named layer probe. Do not extract these nodes or build
the broader vocabulary merely because the first swarm slice exists. Human
listening must decide whether this particular compiled instrument has value
before any vocabulary expansion. Production integration is intentionally
limited to the selected fixed swarm graph.
