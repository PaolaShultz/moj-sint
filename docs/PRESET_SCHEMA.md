# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 10 adds monophonic Open303 and its explicit filter identity while retaining the existing model-specific
identity and independent `instrument_volume`. Every preset has
`schema_version`, `name`, `voices`, `output_gain`, `instrument_volume`,
`model`, one matching patch/core field, and one exact `macros` or `controls`
table. Unknown,
mixed-model, or missing fields fail validation. Names must be non-empty,
voices are 1–64, and all gain, volume, and macro values are finite in `0..=1`.

The eight model identities are:

| Model | Patch field and accepted IDs |
| --- | --- |
| `model_d` | `model_d_patch`: `bass`, `lead`, `filter_articulation` |
| `six_op_pm` | `six_op_patch`: `bell_metal`, `fractured_metal`, `electric_piano_mallet`, `glass_wood`, `brass_bass`, `mechanical_stab` |
| `strange_oscillator` | `strange_patch = "unified"` |
| `swarm_machine` | `swarm_patch = "warm_pad"` |
| `bass_matrix` | `bass_matrix_patch = "transformer"` |
| `dual_filter` | `dual_filter_core`: `industrial`, `counter` |
| `open303` | `open303_filter`: `tb303`, `lowpass18`; `voices = 1` only |
| `pressure_chain` | `pressure_chain_topology`: `deep_cascade`, `body_tap`, `cross_feed`; `voices = 1` only |

```toml
schema_version = 7
name = "16 Bass Matrix"
voices = 4
output_gain = 0.46
instrument_volume = 1.0
model = "bass_matrix"
bass_matrix_patch = "transformer"

[macros]
body = 0.66
growl = 0.18
metal = 0.08
punch = 0.42
character = 0.5
drive = 0.2
filter = 0.48
unstable = 0.08
attack = 0.005
decay = 0.2
sustain = 0.82
release = 0.15
```

Schemas 1–9 remain readable through strict migrations. Older presets gain
`instrument_volume = 1.0`, so their previous full-level behavior and timbre
remain unchanged. The serializer always writes schema 10 and retains the exact
model-specific patch and macro vocabulary.

## Physical positions

Position 5 is instrument volume for the first five models. Moj Sint receives it as MIDI
CC 7 and smooths the linear gain over 10 ms after synthesis, so it changes
level without changing tone. The other seven timbre positions and four ADSR
positions remain model-specific:

| Position | Model D | Six-Op PM | Strange | Swarm | Bass Matrix |
| ---: | --- | --- | --- | --- | --- |
| 1 | `EVOLVE` | `INDEX` | `TYPE` | `MASS` | `BODY` |
| 2 | `SHAPE` | `RATIO` | `FORM` | `DETUNE` | `GROWL` |
| 3 | `COLOR` | `FEEDBACK` | `WARP` | `SPREAD` | `METAL` |
| 4 | `EDGE` | `OP DECAY` | `COUPLE` | `SHAPE` | `PUNCH` |
| 5 | `VOLUME` | `VOLUME` | `VOLUME` | `VOLUME` | `VOLUME` |
| 6 | `MOTION` | `KEY SCALE` | `CHAOS` | `MOTION` | `DRIVE` |
| 7 | `DEPTH` | `VELOCITY` | `COLOR` | `COLOR` | `FILTER` |
| 8 | `SPACE` | `MOTION` | `SPACE` | `SPACE` | `UNSTABLE` |
| 9–12 | `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` | same | same | same | same |

The schema retains the historical fifth timbre macro (`couple`, `balance`,
`motion`, `bite`, or `character`) so old automation and exact sound identity
remain loadable. The SHR surface no longer assigns that slot to position 5;
new physical performance uses the separate `instrument_volume` field.

Dual Filter alone uses all 15 positions and owns both its filter and amp
envelopes internally:

| Position | Dual Filter |
| ---: | --- |
| 1–3 | `FILTER A CUTOFF`, `FILTER A RESONANCE`, `FILTER A ENVELOPE DEPTH` |
| 4–6 | `FILTER B CUTOFF`, `FILTER B RESONANCE`, `FILTER B ENVELOPE DEPTH` |
| 7 | `STRUCTURE` (serial to parallel in INDUSTRIAL; routing/growl macro in COUNTER) |
| 8–11 | filter `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` |
| 12–15 | amp `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` |

MIDI CC20–34 carry those positions. CC35 is a press-only reversible core
toggle; CC36 restores exact core state (`0` INDUSTRIAL, `127` COUNTER). Current
positions are preserved across the internal 30 ms held-note crossfade.

All live controls use the bounded event path. Timbre and ADSR use the existing
10 ms macro smoothers; volume has its own 10 ms smoother. Loading or RESET in
SHR re-arms pickup against the loaded values.

The tracked catalog contains seven Model D, six Six-Op PM, one Strange
Oscillator, one Swarm Machine, one Bass Matrix, five Dual Filter starts, three Pressure Chain starts, and four Open303 starts. The graph description
in `experiments/swarm-micro-machine-v1.toml` remains a strict authoring input;
the live `swarm_machine` model compiles that graph before audio rendering and
never parses or allocates in the callback.

## Pressure Chain

Schema 9's `macros` are `source`, `shape`, `cutoff`, `resonance`, `sweep`,
`filter_decay`, `pressure`, `bite`, `attack`, `decay`, `sustain`, `release`.
Those twelve values map in order to CC20–31. SHR displays F DECAY separately
from amp DECAY; physical position 5 remains SWEEP for this model. CC7 still
controls independent instrument volume at the MIDI host boundary. SHR's last
three performance slots are Project AUX sends, outside the synth.

The three topology presets use the same controls and conservative 0.7 output
gain. Overlapping notes slide without retriggering either contour; releasing
the latest note returns to the most recently held note. A detached note
retriggers both contours, and PANIC clears held keys and voice state. This
model owns its amp envelope internally, so Engine applies no second ADSR.

## Open303 native surface

Open303 has eleven normalized macro fields and independent instrument volume.
Its twelve physical positions are Waveform, Cutoff, Resonance, Env Mod, Volume,
Filter Decay, Accent, Slide, Normal Attack, Accent Attack, Accent Decay, and
Amp Decay. CCs are 20–23, 7, and 25–31 respectively; CC24 is unused. Positions
13–15 remain SHR AUX sends. The last four controls are native envelope timings,
not ADSR. Exact fields/ranges and the four starts are in
[Open303 integration](OPEN303_INTEGRATION.md). Filter identity is preset-owned.
