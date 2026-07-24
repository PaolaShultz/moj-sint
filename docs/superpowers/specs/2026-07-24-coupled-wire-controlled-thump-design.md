# Coupled Wire controlled-thump design

Date: 2026-07-24

## Decision

Develop one successor to the original 1.6-second Coupled Wire reference. Keep
the sub and bass clean, centered, and controlled. Move the deliberately nasty
part of the strike into a short upper-bass branch. Do not make another
multi-variation batch and do not extend this work into `Engine`, presets, or
stable macros.

The current four-file envelope/motion batch is rejected. Human listening found
only its exact first reference somewhat usable. At high line-output gain that
reference exposed excessive bass/sub distortion. The three longer envelope
developments are not retained.

## Observed failure

The reference WAV is not merely exposing an external playback-chain problem.
Its presentation path multiplies the complete stereo signal by 4.43 and then
hard-clamps it at `0.96605086`, or -0.3 dBFS. At 48 kHz:

- 792 channel samples touch the ceiling;
- ceiling contact runs from 8.458 to 79.958 ms;
- the 0-100 ms complete-signal RMS is about -4 dBFS;
- the 45-90 Hz onset band is -7.456 dBFS RMS; and
- the complete file has 0.516% ceiling contact.

Raising line-output gain can therefore reveal both the harmonics already
created by the full-band digital clamp and further analogue overload from the
strong 73 Hz body. The software cannot guarantee headroom in an unknown
external playback chain, but it can stop generating full-band clipped bass
and leave modest digital peak margin.

## Signal path

Start from the unpresented original Coupled Wire generator, before its 4.43
gain and full-band clamp. Preserve its D2 pitch, Cross-derived 7 ms exciter,
mode ratios and losses, two-bank detune and delay, coupling, pickup weights,
natural 1.6-second decay, and prepared 35 Hz DC control.

Process each frame through prepared, complementary scalar branches:

1. Derive a centered mono low body below 105 Hz. This branch never enters a
   nonlinear function. During the strike it follows a deterministic contour
   that reduces its first 80 ms by 1.5-3 dB and returns smoothly to unity by
   150 ms.
2. Derive a 105-500 Hz thump branch from the complementary residual. Its
   odd-symmetric piecewise hard-crest contribution begins at 8 ms, remains
   active through the 45 ms strike window, and fades completely to zero by
   70 ms. Blend no more than 35% shaped signal at maximum.
3. Keep the residual above 500 Hz clean so the selected brightness is not
   turned into broadband fizz.
4. Recombine the centered low body with the original channel-specific upper
   branches. Apply prepared DC control after the nonlinear branch.
5. Choose one fixed presentation gain that keeps sample peak at or below
   -2 dBFS and measured true peak at or below -1.5 dBTP. A final safety clamp
   may exist only if tests prove that it touches zero samples in the retained
   render.

The 105 Hz boundary lies between the 73.416 Hz fundamental and the roughly
147 Hz second mode. The 500 Hz boundary contains the upper-bass knock while
leaving the established bright partial structure mostly outside the nonlinear
path. Complementary subtraction keeps the clean branch sum defined rather
than relying on unrelated filters that can create a crossover hole.

## Configuration selection

Engineering may sweep a small predeclared set of thump-branch drive, threshold,
and wet-blend values. This is an internal bounded sweep, not a listening batch.
Select the least nonlinear configuration that satisfies every contract below.
Write the selected values and all rejected configurations to reports.

Do not add a compressor, adaptive level detector, full-band saturation,
automatic per-file normalization, reverb, delay, chorus, pitch vibrato, or
parallel dry source. Do not change the source mechanism to make a metric pass.

## Automated contracts

Tests and the offline lab must prove:

- deterministic, finite, bounded output at 48 kHz;
- allocation-free sampling with all coefficients and contours prepared before
  the sample loop;
- the original unpresented Coupled Wire source remains sample-identical before
  the new presentation stage;
- the low stem never enters the shaper and its nonlinear residual below
  105 Hz remains at least 40 dB below the clean low stem;
- 45-90 Hz onset RMS is 1.5-4 dB below the rejected reference, controlling the
  overload without hollowing out the fundamental;
- shaped contribution begins no earlier than 8 ms, ends its hard-crest window
  by 45 ms, and reaches exact zero by 70 ms;
- the 105-500 Hz early nonlinear residual is between -30 and -12 dB relative
  to that branch, making the thump present but bounded;
- no final full-band sample reaches a hard ceiling;
- whole-file RMS remains between -15.5 and -11.5 dBFS;
- sample peak is at most -2 dBFS and true peak is at most -1.5 dBTP;
- DC, maximum jump, tonal coverage, spectral flatness, natural decay, final
  zero, stereo correlation, mono loss, and low-band mono retention pass
  explicit rejection bounds;
- the candidate is rejected as noise when flatness and lost tonal anchors
  jointly fail;
- eight-times-rate comparison measures the nonlinear path's high-rate
  residual, which must improve by at least 3 dB over the rejected full-band
  clamp; and
- two independent release renders are byte-identical except documented
  workstation timing.

The reports must separate measured digital level from acoustic SPL. They must
state that external line-output, amplifier, loudspeaker, room, and listening
level can still overload independently of the rendered waveform.

## Listening artifact and lifecycle

After all automated contracts pass, replace
`artifacts/coupled-wire-envelope-motion/` with
`artifacts/coupled-wire-controlled-thump/`. The successor directory contains
exactly one WAV, `01_coupled_wire_controlled_thump.wav`, plus measurement,
selection, rejection, hash, and workstation-cost reports.

Move the rejected envelope/motion batch to Trash only after the passing
successor is installed. If no configuration passes, delete the failed
generation and present no WAV. Human listening remains the only musical
acceptance gate.

## Verification and boundaries

Follow test-first development. Because this changes a nonlinear render path,
include finite-output, allocation, deterministic, alias/high-rate-residual,
band-isolation, and mono-fold coverage before implementation.

Before handoff run formatting, all-target/all-feature tests, Clippy with
warnings denied, release build, audit, deny, AArch64 compile-only validation,
two independent release generations, deterministic comparison, and artifact
hygiene checks.

Do not touch JACK, ALSA, hardware, SHR-DAW, production `Engine`, presets, or
stable macros. Make no Raspberry Pi performance, latency, polyphony, or sound
quality claim.
