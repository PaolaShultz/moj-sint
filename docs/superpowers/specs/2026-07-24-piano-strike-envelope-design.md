# Piano-Strike Envelope Audition Design

Date: 2026-07-24

## Status

Approved for implementation by the user on 2026-07-24.

## Problem

The rejected monophonic envelope batch used a flat `0.58` sustain for most of
each 2.4-second file. It therefore sounded like a held steady tone even though
the attack value changed. The user wants a shorter piano-like amplitude shape
and a brief mixed tone at note onset, analogous to a hammer or bat strike.
That short tone must be varied rather than presenting only attack-time
positions of one unchanged graph.

The rejected generated batch has been moved to Trash. No active listening
artifact remains.

## Scope

This experiment remains isolated from `Engine`, presets, stable macros,
JACK/ALSA, SHR-DAW, and Raspberry Pi claims. It reuses only the exact original
single-note D2 sources already present in the successful twelve-file pool:

- `CrossSingle`;
- `SpectralSingle`; and
- `DualSingle`.

The common body is the same equal-power-trimmed, fixed-microdelay composite
used by the prior test. Only the short pitched strike source changes among the
three listening files.

## Common Body

Every file is musically monophonic and retains the source renderers' stereo
channels. The body:

- sums `CrossSingle`, `SpectralSingle`, and `DualSingle`;
- uses fixed `0/2/5 ms` source offsets;
- keeps the original D2 pitch, DSP, deterministic seeds, and stereo
  construction;
- uses a `2 ms` rise from zero to full level;
- falls from `1.0` to `0.28` over the following `60 ms`;
- continues as a curved decay from `0.28` to zero over `638 ms`; and
- remains exactly zero for the final `100 ms` of an `800 ms` file.

There is no flat sustain stage and no note-off release stage. The body is a
bounded one-shot piano-like decay, not an ADSR hold.

## Varied Strike Layer

Each file adds one short, pitched D2 strike made from an exact original source:

| profile | strike source | strike length | character under test |
| --- | --- | ---: | --- |
| Cross strike | `CrossSingle` | `16 ms` | tight and hard |
| Spectral strike | `SpectralSingle` | `28 ms` | bright and metallic |
| Dual strike | `DualSingle` | `42 ms` | round and resonant |

Each strike rises over `1 ms`, then follows a curved decay to exact zero at its
declared end. Its fixed pre-presentation mix coefficient is `0.30`. There is
no noise burst, unpitched click, newly authored oscillator, new effect, or
separately audible delayed event.

This creates three fundamentally different transient-source mechanisms while
holding the body, note, body envelope, duration, and presentation policy
constant.

## Presentation and Rejection

All three previews select one shared fixed presentation gain. There is no
per-file normalization, compressor, automatic limiter, soft saturator, or
time-varying make-up gain. A static `-0.3 dBFS` ceiling may catch sparse crests;
ceiling contact must remain at or below `1%`.

Whole-file stereo RMS includes the complete `800 ms`, including the final
silence. A candidate below `-14 dBFS` whole-file RMS is rejected and receives
no WAV. The shared-gain selector must also keep every candidate at or below
`-10 dBFS` whole-file RMS.

A WAV is also rejected for non-finite output, excessive DC, excessive
adjacent-sample jump, stereo/mono failure, failed tonal coverage, conjunctive
noise-like classification, missing final silence, or excessive ceiling
contact.

## Listening Batch

The only active artifact directory will be:

`artifacts/piano-strike-envelope-composites/`

It may contain at most:

1. `01_piano_cross_strike.wav`;
2. `02_piano_spectral_strike.wav`; and
3. `03_piano_dual_strike.wav`.

The directory also contains the README, manifest, metrics, rejection report,
hashes, generation summary, and workstation-cost report. A rejected profile
is reported but its WAV is not written. No old envelope batch, chord,
progression, solo, diagnostic WAV, or failed WAV may remain.

## Tests and Evidence

Test-first coverage must prove:

- the common body has a `2 ms` attack, `60 ms` fast decay, no flat sustain,
  curved decay to zero by `700 ms`, and exact final silence;
- the three profiles use the exact Cross, Spectral, and Dual single-note
  sources with `16/28/42 ms` strike lengths;
- strike layers reach exact zero and cannot become delayed separate events;
- sampling remains deterministic, finite, bounded, and allocation-free;
- one shared gain satisfies the whole-file RMS and sparse-ceiling constraints;
- a deliberately weak whole-file profile is rejected before WAV writing;
- exactly the three declared files are eligible for the listening directory;
  and
- two release generations are byte-identical except the volatile
  workstation-cost file.

The complete repository verification remains `cargo fmt --check`, all-target
tests, Clippy with warnings denied, release build, `cargo audit`,
`cargo deny check`, AArch64 compile check, deterministic release generation,
and artifact hygiene. Automated evidence rejects obvious failures; human
listening remains the musical acceptance gate.
