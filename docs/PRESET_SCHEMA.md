# Preset and Control Schema

`.mojsint` is strict TOML. Schema version 3 contains only these top-level
fields: `schema_version`, `name`, `voices`, `output_gain`, `model_d_patch`, and
`macros`. Unknown or missing fields fail validation. Presets must be regular
UTF-8 files, names must be non-empty, voices are 1–64, and gain/macro values
are finite numbers in `0..=1`. `model_d_patch` is exactly `bass`, `lead`, or
`filter_articulation`.

```toml
schema_version = 3
name = "04 Matched Idealized"
voices = 8
output_gain = 0.2
model_d_patch = "bass"

[macros]
evolve = 0.0
shape = 0.5
color = 0.4125
edge = 0.0
couple = 0.0
motion = 0.43333334
depth = 0.0
space = 0.45333335
attack = 0.18092207
decay = 0.59434659
sustain = 0.7
release = 0.59434659
```

Schema versions 1 and 2 remain readable through strict migrations. Version 2
becomes the bass patch. Version 1 additionally ignores its redundant
`envelope` table in favor of the macro ADSR values and discards provisional
`width`. In-memory identity becomes version 3; new presets must use version 3.

| CC | Macro | Model D behavior |
| ---: | --- | --- |
| 20 | `EVOLVE` | 0 idealized; 0.5 authored static offset/asymmetry/level without drift; 1 full authored oscillator character |
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
Off. The seven tracked presets preserve the authored starting coordinates from
the Model D audition: three full patches and four matched bass diagnostics.
Focused tests prove the five bass macro coordinates reproduce the corresponding
diagnostic render and that all seven live starts remain finite and pairwise
distinct. There is no thirteenth control, hidden page, or master-encoder mode.
`WIDTH` was removed because the production Model D engine is intentionally
dual-mono and no width experiment exists in that model.
