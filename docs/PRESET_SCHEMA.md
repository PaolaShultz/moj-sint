# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 6 keeps the Moj Sint process
(`engine`) separate from its synthesis model and makes the patch field and
macro table model-specific. Every preset has `schema_version`, `name`,
`voices`, `output_gain`, `model`, one matching patch field, and `macros`.
Unknown, mixed-model, or missing fields fail validation. Presets must be
regular UTF-8 files, names must be non-empty, voices are 1–64, and gain/macro
values are finite numbers in `0..=1`.

Model D uses `model = "model_d"`, `model_d_patch`, and its established macros.
Six-Op PM uses `model = "six_op_pm"`, `six_op_patch`, and the second macro
shape. The six patch IDs are `bell_metal`, `fractured_metal`,
`electric_piano_mallet`, `glass_wood`, `brass_bass`, and `mechanical_stab`.
Strange Oscillator uses `model = "strange_oscillator"`,
`strange_patch = "unified"`, and its own eight macro names.

```toml
schema_version = 6
name = "08 Six-Op Bell Metal"
voices = 4
output_gain = 0.8
model = "six_op_pm"
six_op_patch = "bell_metal"

[macros]
index = 0.5
ratio = 0.5
feedback = 0.5
operator_decay = 0.5
balance = 0.5
key_scale = 0.5
velocity = 0.5
motion = 0.5
attack = 0.05
decay = 0.35
sustain = 0.8
release = 0.25
```

Schema versions 1–5 remain readable through strict migrations. Version 4
already carries `model = "model_d"`; versions 1–3 gain that identity during
migration, while version 5 retains Model D and Six-Op PM exactly. In-memory
identity becomes version 6. New presets must use version 6. The library's
`Preset::to_toml` serializer
always writes that strict current schema: migrated Model D sounds retain the
matching `model_d_patch` and macro names, while Six-Op PM retains
`six_op_patch` and its exact model-specific macro vocabulary. Strange
Oscillator retains `strange_patch = "unified"` and its exact macros.

The physical positions stay CC 20–31. Their meaning comes from the loaded
model:

| CC | Model D | Six-Op PM | Strange Oscillator |
| ---: | --- | --- | --- |
| 20 | `EVOLVE` — oscillator character and drift | `INDEX` — phase-modulation depth | `TYPE` — source topology |
| 21 | `SHAPE` — source balance | `RATIO` — modulator-ratio spread | `FORM` — topology-specific structure |
| 22 | `COLOR` — ladder cutoff | `FEEDBACK` — delayed-feedback amount | `WARP` — contour and harmonic deformation |
| 23 | `EDGE` — mixer character and drive | `OP DECAY` — operator-envelope time and live decay emphasis | `COUPLE` — cross/ring interaction |
| 24 | `COUPLE` — output feedback | `BALANCE` — carrier/modulator balance | `MOTION` — cyclic movement and rate |
| 25 | `MOTION` — filter-contour amount | `KEY SCALE` — keyboard-brightness response | `CHAOS` — held-cycle disruption |
| 26 | `DEPTH` — ladder character and drive | `VELOCITY` — operator velocity response | `COLOR` — harmonic and tone travel |
| 27 | `SPACE` — ladder resonance | `MOTION` — pitch-envelope and LFO movement | `SPACE` — mid/side width |
| 28 | `ATTACK` | `ATTACK` | `ATTACK` |
| 29 | `DECAY` | `DECAY` | `DECAY` |
| 30 | `SUSTAIN` | `SUSTAIN` | `SUSTAIN` |
| 31 | `RELEASE` | `RELEASE` | `RELEASE` |

All twelve values use the live 10 ms smoothing path. The outer ADSR is live
for all models, including release changes made after Note Off. Six-Op PM
timbral controls affect held notes; operator-envelope time itself is prepared
from the smoothed value when the next note starts. Neutral `0.5` timbral values
preserve each authored six-operator patch.

The tracked catalog contains seven Model D starts, six Six-Op PM starts, and
one Strange Oscillator start. There is no thirteenth control, hidden page, or
master-encoder mode. Model D remains intentionally dual-mono and has no width
control; Strange Oscillator generates model-owned stereo.
