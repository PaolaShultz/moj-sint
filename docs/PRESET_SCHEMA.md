# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 7 keeps the Moj Sint process
(`engine`) separate from its synthesis model, makes patch identity and macros
model-specific, and adds `instrument_volume`. Every preset has
`schema_version`, `name`, `voices`, `output_gain`, `instrument_volume`,
`model`, one matching patch field, and one exact `macros` table. Unknown,
mixed-model, or missing fields fail validation. Names must be non-empty,
voices are 1–64, and all gain, volume, and macro values are finite in `0..=1`.

The five model identities are:

| Model | Patch field and accepted IDs |
| --- | --- |
| `model_d` | `model_d_patch`: `bass`, `lead`, `filter_articulation` |
| `six_op_pm` | `six_op_patch`: `bell_metal`, `fractured_metal`, `electric_piano_mallet`, `glass_wood`, `brass_bass`, `mechanical_stab` |
| `strange_oscillator` | `strange_patch = "unified"` |
| `swarm_machine` | `swarm_patch = "warm_pad"` |
| `bass_matrix` | `bass_matrix_patch = "transformer"` |

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

Schemas 1–6 remain readable through strict migrations. Older presets gain
`instrument_volume = 1.0`, so their previous full-level behavior and timbre
remain unchanged. The serializer always writes schema 7 and retains the exact
model-specific patch and macro vocabulary.

## Twelve physical positions

Position 5 is instrument volume for every model. Moj Sint receives it as MIDI
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

All live controls use the bounded event path. Timbre and ADSR use the existing
10 ms macro smoothers; volume has its own 10 ms smoother. Loading or RESET in
SHR re-arms pickup against the loaded values.

The tracked catalog contains seven Model D, six Six-Op PM, one Strange
Oscillator, one Swarm Machine, and one Bass Matrix start. The graph description
in `experiments/swarm-micro-machine-v1.toml` remains a strict authoring input;
the live `swarm_machine` model compiles that graph before audio rendering and
never parses or allocates in the callback.
