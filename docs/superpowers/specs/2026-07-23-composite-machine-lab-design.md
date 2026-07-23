# Composite Machine Lab Design

Date: 2026-07-23

## Goal

Turn the first strongly positive Moj Sint listening accident into a controlled,
deterministic offline research boundary. The lab internally instantiates the
three existing hybrid DSP families, schedules heterogeneous note roles, mixes
them with explicit gain/stereo policy, and measures causal contributions. It
does not read the twelve WAVs while rendering, alter production `Engine`, or
touch JACK, ALSA, hardware, SHR-DAW, presets, or stable macros.

The positive verdict remains a direction, not an accepted sound. Automated
evidence may reject unsafe, inert, redundant, or near-identical graphs; only the
user can decide whether a final candidate retains the accident's musical value.

## Corrected reconstruction boundary

The twelve source files were not launched sample-synchronously. Each was
spawned in a separate player process with a noticeable delay. The WAV files
contain no launch timestamps, and no player/process log is available, so the
historical offsets and launch order cannot be recovered exactly.

The lab therefore separates two references:

1. `synchronized-reference` is an exact internal reconstruction of the twelve
   file schedules, seeds, per-file loudness preparation, durations, stereo
   progression layers, and redundant mono-fold progression layers, with every
   start at sample zero. It is a controlled counterfactual, not the accident.
2. `delayed-launch-estimate` uses the same twelve sources with a preregistered,
   deterministic, substantially staggered launch sequence. It tests the
   plausible consequences of separate process startup: density ramp, phase
   dispersion, beating, and shifted end events. It is the closest reproducible
   model available, but it is explicitly an estimate.

For the synchronized reference, 0-4 seconds contain three MIDI-38 voices and
nine D-minor three-note ensembles: 30 synthesis voices before internal
partials, resonators, register state, or stereo paths. The delayed estimate
must report its actual time-varying inventory instead of repeating that claim.

## Architecture

Create an isolated `composite_machine` library module and
`composite-machine-lab` binary.

`CompositeMachine` is constructed from immutable layer specifications. Each
layer declares:

- hybrid family and score role;
- deterministic seed/phase policy;
- sample-accurate start and stop;
- prepared per-source loudness gain and candidate mix gain;
- a 2x2 stereo matrix;
- bounded start/end/segment fades; and
- a stable diagnostic label.

Construction may allocate and prepare complete fixed state. Sampling iterates
fixed layer storage, performs no allocation/deallocation, I/O, locking,
logging, formatting, process work, clocks, or avoidable transcendental setup,
and writes no emergency limiter. Candidate gains reserve headroom before
rendering. The existing hybrid voices retain their own bounded internal output
guards.

The non-real-time renderer may allocate output and reports. It can render
mute-one ablations, mono folds, and high-rate comparisons from fresh
deterministic machine instances.

## Delayed-launch estimate

Because exact offsets are unknowable, use a fixed filename-order hypothesis
with offsets in milliseconds:

```text
0, 180, 420, 710,
1030, 1380, 1760, 2170,
2610, 3080, 3580, 4110
```

The order is cross-coupled single/chord/progression/progression-mono, then
spectral-shadow in the same order, then dual-resonant-body. The schedule is not
claimed as the user's actual click/process order. It deliberately spans more
than four seconds so the diagnostic actually represents "not so small" launch
delays.

## Final topology set

### 1. Synchronized reference

All twelve original source schedules begin together. Per-file normalization
matches the old lab preparation, then a fixed composite gain provides safe
headroom. This is the only exact twelve-schedule reconstruction.

### 2. Delayed-launch estimate

The same layers use the preregistered start offsets. No other topology or gain
changes are permitted, so differences isolate scheduling and phase/density
effects.

### 3. Reduced heterogeneous stack

Start from the accidental ingredients, remove all progression-mono duplicates,
and retain only layers whose mute ablation materially changes level, spectrum,
stereo, or target-note structure. Keep at least two families and do not pursue
minimum layer count until contribution evidence exists.

### 4. Role-separated compound machine

Assign distinct causal jobs: cross-coupled machinery owns the centered D2
anchor, spectral shadow owns the evolving harmonic object, and dual resonant
body supplies transition excitation and stored stereo energy. This uses custom
long-form roles rather than three copies of one score.

### 5. Harmonic-lattice machine

Hold D2 as a stable low lattice point while individual heterogeneous voices
carry changing lower, middle, and upper chord relationships. Harmony is inside
the timbre construction; there is no conventional full backing-chord layer.

### 6. Cross-topology follower machine

A fixed attack/release envelope follower measures one family's mid signal and
modulates another family's bounded character gain and a third family's side
placement. This hardwired typed connection tests:

```text
AudioStereo -> Mid -> Measurement(envelope)
Measurement -> bounded Control gain
Measurement -> bounded Stereo matrix
```

The follower cannot mute a layer, exceed declared gain limits, or chatter.

### 7. Risky nonlinear braid

Add a low-level bounded multiplicative junction between two per-family signals
while retaining a dry harmonic anchor. This is deliberately different from a
linear stack and is labeled as a post-source composite nonlinear hypothesis.
It must receive direct difference/sum-product, chord-readability, DC, jump, and
high-rate residual evidence. Reject it if the junction dominates, produces
uncontrolled low difference tones, or erases target-note relationships.

## Evidence and rejection rules

Predeclare these engineering failures:

- any non-finite sample, sample-path allocation, or output peak above `0.8`;
- absolute full-render DC above `5e-4`;
- claimed stereo with correlation above `0.995` or side/mid below `0.005`;
- mono-fold RMS below half stereo RMS or loss of any scheduled target by more
  than 36 dB relative to the strongest scheduled target in that segment;
- any layer whose mute-one difference RMS is below -42 dB relative to the
  candidate, unless it has a separately measured sparse transient role;
- pairwise gain-fitted similarity above `0.985` for two claimed final
  candidates, unless their time inventory is intentionally the only variable
  as in the two references;
- follower movement that changes its destination gain by less than 3%;
- a nonlinear junction whose strongest unintended difference product is less
  than 30 dB below the weakest scheduled target;
- severe unexplained high-rate residual or sample-rate-dependent event timing;
- uncontrolled normalization, clipping, or a final limiter hiding the mix.

Reports cover whole renders and musically meaningful event intervals:
peak/RMS/DC/crest/jump; low/mid/high energy; brightness balance and band flux;
target, octave, subharmonic, and sparse-spine projections; onset/body timing;
full and low-band stereo/mono evidence; per-layer mute ablation; pairwise
similarity; nonlinear products; conservative high-rate residual; hashes; and
scalar workstation cost. Cost is not Raspberry Pi or callback evidence.

## Iteration and output policy

Generate an initial temporary batch, inspect every report, document rejected
graphs/layers, delete it, revise the topology or gains, and generate a fresh
batch. Repeat deterministic comparison after redesign.

The final ignored batch lives only at
`artifacts/composite-machine-lab/`. It contains the two references, four to six
surviving candidate WAVs, explicit mono diagnostics, a concise listening
README, and the complete reports. Generated output is not committed.

## Human gate

Suggested order:

1. synchronized counterfactual;
2. delayed-launch estimate;
3. reduced heterogeneous stack;
4. role-separated compound machine;
5. harmonic lattice;
6. cross-topology follower;
7. risky nonlinear braid;
8. mono diagnostics.

The README must state that digital normalization/headroom does not measure
acoustic SPL or safe playback level. After listening, the user decides which
connections, if any, deserve typed-node extraction. Nothing enters production
or the generic graph runtime before that verdict.
