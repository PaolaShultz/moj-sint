# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 5 keeps the Moj Sint process
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

```toml
schema_version = 5
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

Schema versions 1–4 remain readable through strict migrations and always
select Model D. Version 4 already carries `model = "model_d"`; versions 1–3
gain that identity during migration. In-memory identity becomes version 5.
New presets must use version 5. The library's `Preset::to_toml` serializer
always writes that strict current schema: migrated Model D sounds retain the
matching `model_d_patch` and macro names, while Six-Op PM retains
`six_op_patch` and its exact model-specific macro vocabulary.

The physical positions stay CC 20–31. Their meaning comes from the loaded
model:

| CC | Model D | Six-Op PM |
| ---: | --- | --- |
| 20 | `EVOLVE` — oscillator character and drift | `INDEX` — phase-modulation depth |
| 21 | `SHAPE` — source balance | `RATIO` — modulator-ratio spread |
| 22 | `COLOR` — ladder cutoff | `FEEDBACK` — delayed-feedback amount |
| 23 | `EDGE` — mixer character and drive | `OP DECAY` — operator-envelope time and live decay emphasis |
| 24 | `COUPLE` — output feedback | `BALANCE` — carrier/modulator balance |
| 25 | `MOTION` — filter-contour amount | `KEY SCALE` — keyboard-brightness response |
| 26 | `DEPTH` — ladder character and drive | `VELOCITY` — operator velocity response |
| 27 | `SPACE` — ladder resonance | `MOTION` — pitch-envelope and LFO movement |
| 28 | `ATTACK` | `ATTACK` |
| 29 | `DECAY` | `DECAY` |
| 30 | `SUSTAIN` | `SUSTAIN` |
| 31 | `RELEASE` | `RELEASE` |

All twelve values use the production 10 ms smoothing path. The outer ADSR is
live for both models, including release changes made after Note Off. Six-Op PM
timbral controls affect held notes; operator-envelope time itself is prepared
from the smoothed value when the next note starts. Neutral `0.5` timbral values
preserve each authored six-operator patch.

The tracked catalog contains seven Model D starts and six Six-Op PM starts.
There is no thirteenth control, hidden page, or master-encoder mode. Model D
remains intentionally dual-mono and has no width control.
