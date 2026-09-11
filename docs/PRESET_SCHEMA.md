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

## Native controls and SHR positions

SHR owns the physical surface: four rows of four, with tone controls on rows
1–2, envelopes on row 3, and Volume/AUX 1/AUX 2/AUX 3 on row 4. Rotary 1 edits
parameter 1; click toggles visible NAV for menu-page selection. The complete
mapping is maintained in
[SHR's instrument guide](https://github.com/PaolaShultz/shr-daw/blob/main/docs/INSTRUMENTS_AND_DRUMS.md#moj-sint-sounds).

Moj receives volume independently as CC7 and smooths linear gain over 10 ms
after synthesis. The following table describes native macro order (CC20–31),
not physical rotary positions:

| Native slot | Model D | Six-Op PM | Strange | Swarm | Bass Matrix |
| ---: | --- | --- | --- | --- | --- |
| 1 | `EVOLVE` | `INDEX` | `TYPE` | `MASS` | `BODY` |
| 2 | `SHAPE` | `RATIO` | `FORM` | `DETUNE` | `GROWL` |
| 3 | `COLOR` | `FEEDBACK` | `WARP` | `SPREAD` | `METAL` |
| 4 | `EDGE` | `OP DECAY` | `COUPLE` | `SHAPE` | `PUNCH` |
| 5 | `COUPLE` | `BALANCE` | `MOTION` | `BITE` | `CHARACTER` (preset-only) |
| 6 | `MOTION` | `KEY SCALE` | `CHAOS` | `MOTION` | `DRIVE` |
| 7 | `DEPTH` | `VELOCITY` | `COLOR` | `COLOR` | `FILTER` |
| 8 | `SPACE` | `MOTION` | `SPACE` | `SPACE` | `UNSTABLE` |
| 9–12 | `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` | same | same | same | same |

SHR restores Model D feedback, Six-Op balance, Strange motion and Swarm bite
on the surface. Bass Matrix retains `character` for preset compatibility but
its live macro path ignores that field; SHR leaves the spare cell empty.

Dual Filter owns fifteen native controls plus independent CC7 volume. SHR
keeps both filters' cutoff/resonance/envelope amount, structure, filter decay
and amp ADSR on its main surface. Filter attack/sustain/release remain saved
preset detail rather than a second AMP/FILTER page.

| Native slot | Dual Filter |
| ---: | --- |
| 1–3 | `FILTER A CUTOFF`, `FILTER A RESONANCE`, `FILTER A ENVELOPE DEPTH` |
| 4–6 | `FILTER B CUTOFF`, `FILTER B RESONANCE`, `FILTER B ENVELOPE DEPTH` |
| 7 | `STRUCTURE` (serial to parallel in INDUSTRIAL; routing/growl macro in COUNTER) |
| 8–11 | filter `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` |
| 12–15 | amp `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE` |

MIDI CC20–34 carry those native controls. CC35 is a press-only reversible core
toggle; CC36 restores exact core state (`0` INDUSTRIAL, `127` COUNTER). Current
values are preserved across the internal 30 ms held-note crossfade.

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
from amp DECAY; physical rotary 5 is SWEEP and rotary 13 is Volume. The final
three rotaries are SHR Project AUX sends, outside the synth.

The three topology presets use the same controls and conservative 0.7 output
gain. Each positive live NoteOn retriggers both contours, including repeated
held keys. The amp attack begins at its current level, and overlapping pitches
still glide. Releasing the latest note returns to the most recently held note
without retriggering. Detached notes start at the new pitch even during the
previous release tail. PANIC clears held keys and voice state. This model owns
its amp envelope internally, so Engine applies no second ADSR.

## Open303 native surface

Open303 has eleven normalized macro fields and independent instrument volume.
Its native controls are Waveform, Cutoff, Resonance, Env Mod, Filter Decay,
Accent, Slide, Normal Attack, Accent Attack, Accent Decay, and Amp Decay.
CCs are 20–23 and 25–31 respectively; CC24 is unused and CC7 is Volume.
SHR places Filter Attack, Filter Decay, Accent Decay and Amp Decay on row 3,
then Volume and three AUX sends on row 4. These are native envelope timings,
not ADSR. Exact fields/ranges and the four starts are in
[Open303 integration](OPEN303_INTEGRATION.md). Filter identity is preset-owned.
