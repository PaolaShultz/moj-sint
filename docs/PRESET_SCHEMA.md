# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 2 contains only these top-level
fields: `schema_version`, `name`, `voices`, `output_gain`, and `macros`.
Unknown or missing fields fail validation. Presets must be regular UTF-8 files,
names must be non-empty, voices are 1–64, and gain/macro values are finite
numbers in `0..=1`.

```toml
schema_version = 2
name = "Model D Baseline"
voices = 8
output_gain = 0.2

[macros]
evolve = 0.0
shape = 0.5
color = 0.4125
edge = 0.0
couple = 0.0
motion = 0.43333334
depth = 0.0
space = 0.45333335
attack = 0.18094
decay = 0.59428
sustain = 0.7
release = 0.59428
```

Schema version 1 remains readable through a strict migration. Its redundant
`envelope` table is ignored in favor of the macro ADSR values, and its
provisional `width` value is discarded. In-memory identity becomes version 2;
new presets must use version 2.

| CC | Macro | Model D behavior |
| ---: | --- | --- |
| 20 | `EVOLVE` | authored oscillator tuning offsets, drift, asymmetry, and level mismatch |
| 21 | `SHAPE` | three-VCO source balance |
| 22 | `COLOR` | ladder cutoff |
| 23 | `EDGE` | linear-to-nonlinear mixer character and drive |
| 24 | `COUPLE` | output feedback |
| 25 | `MOTION` | filter-contour amount |
| 26 | `DEPTH` | linear-to-nonlinear ladder character and drive |
| 27 | `SPACE` | ladder resonance |
| 28 | `ATTACK` | perceptually scaled loudness attack |
| 29 | `DECAY` | perceptually scaled loudness decay |
| 30 | `SUSTAIN` | bounded loudness sustain |
| 31 | `RELEASE` | perceptually scaled loudness release |

All twelve values are smoothed over 10 ms and can affect an active note. ADSR
stage time survives live changes, including release changes made after Note
Off. The reference values reproduce the preferred idealized Model D baseline;
the timbral travel exposes the full modeled experiment space. There is no
thirteenth control, hidden page, or master-encoder mode. `WIDTH` was removed
because the production Model D engine is intentionally dual-mono and no width
experiment exists in that model.
