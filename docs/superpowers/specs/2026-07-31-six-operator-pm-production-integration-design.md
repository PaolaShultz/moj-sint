# Six-Operator PM Production Integration Design

**Date:** 2026-07-31
**Status:** Approved for implementation
**Scope:** Promote the completed six-operator PM experiment into a second playable Moj Sint synthesis model and expose it through SHR-DAW

## Goal

Make the six completed experimental PM sounds playable and editable from
SHR-DAW while preserving Model D, the single owned Moj Sint process, the
existing twelve physical control positions, and strict preset compatibility.

The musician sees one `Moj Sint` backend with two synthesis-model categories:
`Model D` and `Six-Op PM`. Loading a preset starts the same configured
`moj-sint` executable and replaces the currently owned melodic engine exactly
as it does today. This work does not add simultaneous Moj Sint processes or a
fifth SHR-DAW backend.

## Existing work to retain

Merge `feature/six-operator-pm` into Moj Sint `main` without squashing its
verified history. Retain the independently authored fixed-capacity six-operator
graph core, all 32 validated routing shapes, six authored listening patches,
measurements, deterministic lab, tests, and clean-room provenance.

Generated WAV files and reports remain ignored experimental artifacts. The
tracked patch definitions become ordinary production source and six new
`.mojsint` factory presets; no rendered audio is copied into Git.

## Approaches considered

### One Moj Sint backend with two model identities

Extend the model seam already added by Moj Sint 0.2.2 and SHR-DAW 0.4.7. The
same host dispatches either Model D or Six-Op PM based on the loaded preset.
SHR-DAW discovers model-qualified routes and renders model-specific parameter
labels. This preserves process ownership, routing, pickup, Ideas, Projects,
FT2, and failure isolation. **Selected.**

### A separate SHR-DAW backend for Six-Op PM

Treat Six-Op PM as a fifth backend with duplicate executable, configuration,
catalog, routing, and process lifecycle code. Rejected because the Raspberry
Pi changes deliberately separate Moj Sint host identity from synthesis-model
identity.

### Two simultaneous Moj Sint processes

Run Model D and Six-Op PM concurrently. Rejected because SHR-DAW currently
owns one managed melodic engine, and this request is for selectable sounds and
parameters rather than a multi-engine mixer redesign.

## Moj Sint architecture

### Model and preset identity

Add `SixOpPm` to `SynthesisModelId` with stable ID `six_op_pm` and label
`Six-Op PM`. Replace the Model-D-only patch argument at the internal voice
boundary with a model-specific patch enum:

- `ModelD(ModelDPatchId)`
- `SixOpPm(SixOpPatchId)`

`SixOpPatchId` contains the six independently authored experiment starts:

1. Bell Metal
2. Fractured Metal
3. Electric Piano Mallet
4. Glass Wood
5. Brass Bass
6. Mechanical Stab

The strict wire format advances to preset schema 5. A schema-5 document is
deserialized according to its `model` and rejects fields belonging to the
other model. Model D uses `model_d_patch` and its current macro names; Six-Op
PM uses `six_op_patch` and its own macro names. Schemas 1 through 4 remain
strictly readable and migrate to Model D in memory without rewriting files.

### Production voice adapter

Keep the generic PM graph core independent of `Engine`, JACK, ALSA, files,
processes, and clocks. Add a focused production adapter that:

- prepares one shared sine table outside the render callback;
- creates all configured voices and their fixed-capacity state before audio;
- retriggers and retunes an existing voice on note-on without allocation;
- forwards note-off so operator envelopes and the public amplitude envelope
  release coherently;
- applies smoothed live controls without rebuilding graph topology; and
- resets all operator, feedback, envelope, LFO, and diagnostic state on panic
  or voice reuse.

The existing `Engine` continues to own polyphony, deterministic voice stealing,
outer ADSR, stereo duplication, event timing, and output gain. The PM adapter
is one `VoiceModel` variant, not a parallel audio engine.

### Twelve live controls

CC 20 through 31 remain the stable physical positions. Six-Op PM assigns its
first eight positions to continuous, smoothed transformations around each
authored patch; the last four retain the public amplitude ADSR.

| CC | Label | Behavior |
| ---: | --- | --- |
| 20 | Index | Scales declared phase-modulation depth within safe bounds |
| 21 | Ratio | Continuously spreads modulator ratios around the authored values |
| 22 | Feedback | Scales the declared delayed feedback edge |
| 23 | Decay | Scales modulator-envelope times while preserving stage order |
| 24 | Balance | Moves carrier and modulator level balance without graph changes |
| 25 | Key Scale | Scales the authored keyboard-brightness response |
| 26 | Velocity | Scales operator velocity sensitivity |
| 27 | Motion | Scales the authored pitch-envelope and LFO movement |
| 28 | Attack | Public output-envelope attack |
| 29 | Decay | Public output-envelope decay |
| 30 | Sustain | Public output-envelope sustain |
| 31 | Release | Public output-envelope release |

Each factory preset stores a truthful neutral value for every control. At its
neutral coordinates, the production single-note path must reproduce the
corresponding authored PM patch within the declared floating-point tolerance.
Control laws clamp before the sample path and must not introduce per-sample
transcendental setup. The graph algorithm remains preset-owned rather than a
stepped physical control.

## SHR-DAW integration

Extend `MojModel` with `SixOpPm`, stable ID `six_op_pm`, label `Six-Op PM`, and
a model-specific twelve-control table using the labels above. Controller
configuration remains physical positions `POT1` through `POT12`; it does not
gain synthesis parameter names.

SHR-DAW preset discovery accepts Moj Sint schema 5, strictly selects the
model-specific preset shape, and returns all twelve normalized values.
Model-qualified route IDs become:

- `model_d/<preset name>`
- `six_op_pm/<preset name>`

Legacy unqualified lookup remains limited to older Model D Project and Idea
data. Six-Op PM participates in the existing Presets/LOAD, Playback controls,
RESET, pickup, Idea capture, Project persistence, FT2 routing, recording
metadata, and failed-replacement rollback paths. No new process type, JACK
client, ALSA port, audio route, configuration block, screen, button, mode, or
controller bank is added.

## Error handling and compatibility

- Invalid schema, model, model-specific patch, macro, voice count, output
  gain, or non-finite value fails before process launch.
- A failed Six-Op PM load preserves or restores the previous managed engine
  using SHR-DAW's existing replacement transaction.
- Runtime non-finite PM state yields silence for the affected sample and sets
  the existing non-real-time diagnostic; panic resets it.
- Model D schema 1-4 parsing, factory presets, route IDs, controls, sound, and
  host behavior remain unchanged.
- Schema 5 rejects mixed Model D and Six-Op PM fields rather than silently
  ignoring them.

## Test-first verification

Implementation follows red-green-refactor in both repositories.

Moj Sint tests cover schema-5 strict parsing and legacy migration; all six
factory presets; model dispatch; neutral-coordinate reproduction; each live
control's intended effect; note-on, note-off, stealing, reset, and panic;
finite output; allocation-free render/control paths; deterministic output;
rapid controls; parameter corners; and the existing PM alias/error evidence.

SHR-DAW tests cover schema-5 discovery and rejection; `six_op_pm` route IDs;
model-specific control labels and values; controller-position mapping; LOAD
command construction; RESET and pickup; Idea/Project/FT2/recording identity;
legacy Model D routes; rollback; and the one-managed-engine invariant.

Before publication, run the repository-mandated formatting, focused tests,
all-target tests, warning-denied Clippy, locked release builds, audit/deny and
deterministic Moj Sint render comparison. Do not start JACK, Moj Sint, MIDI,
playback, recording, or hardware tests without a separate explicit physical
scope. Do not claim Raspberry Pi performance or final production polyphony
until measured there.

## Publication

Commit coherent Moj Sint integration and verification on `main`, then push it.
Commit the paired SHR-DAW discovery/control/identity integration on its `main`,
then push it. Verify repository, branch, remote, intended commits, and clean
status immediately before each push. Update Moj Sint `docs/HANDOFF.md`,
SHR-DAW `docs/WORKSPACE_HANDOFF.md`, the focused schema/control documentation,
and the concise project knowledge note with verified outcomes only.

## Legal and product boundary

The production model is named `Six-Op PM`, not DX7. It retains the existing
clean-room boundary: independently authored code, graphs, parameter laws, and
presets; no Yamaha firmware, ROM, source, SysEx, factory patch, prose, artwork,
panel design, or branding; and no compatibility or exact-emulation claim.
Historical DX7 references remain provenance for functional facts, not product
identity or copied expression.
