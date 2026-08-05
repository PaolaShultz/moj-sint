# Strange Oscillator Third-Model Gate Design

Date: 2026-08-05

## Goal

Test one candidate third Moj Sint instrument that can move between classic and
strange generator topologies, then transform any selected topology with seven
large structural controls. This is not eight instruments or a bank of lightly
edited presets.

The milestone is an isolated monophonic offline laboratory. It does not add a
production model, preset schema, factory sound, host route, JACK/ALSA behavior,
or SHR-DAW integration. Automated evidence can reject a weak or unsafe mapping;
only listening can accept a useful one.

## Exact control surface

The instrument retains the settled eight timbral controls plus ADSR:

1. `TYPE` — one eight-detent topology selector;
2. `FORM` — source geometry, symmetry, width, or state structure;
3. `WARP` — strong contour and fifth-harmonic deformation;
4. `COUPLE` — interaction with an inharmonic internal partner;
5. `MOTION` — deep cyclic-envelope rate from 0.05 to 50 Hz;
6. `CHAOS` — deterministic whole-cycle grammar and source irregularity;
7. `COLOR` — fundamental-to-fifth-harmonic anchor plus two-pole cutoff; and
8. `SPACE` — mid/side width with a retained mono center.

ADSR remains the four-control outer envelope contract. It is deliberately off
in this dry oscillator gate so it cannot disguise weak timbral mappings.

`TYPE` is quantized into eight stable detents. Runtime changes prepare the new
topology and crossfade for 10 ms while both sample paths remain allocation-free.
The listening reel performs all seven switches on one uninterrupted held note,
so silence or a file edit cannot hide a click.

## Eight TYPE positions

1. **Triangle** — continuous periodic calibration anchor.
2. **Saw** — bandlimited saw-family anchor and the preferred earlier source.
3. **Pulse** — bandlimited pulse-family anchor.
4. **Modulated resonator** — pitched carrier plus a movable modal partial.
5. **Deformed loop** — iterative phase over a smooth non-circular closed path.
6. **Stochastic breakpoints** — a deterministic bounded random walk over one
   cycle's breakpoints.
7. **Scanned string** — a slow dynamic string state scanned at note frequency.
8. **Register machine** — explicit fixed-width wrapping, rotate, carry, borrow,
   and integer-to-audio projection.

These are source topologies inside one `StrangeInstrument`. Neutral hashes are
required to differ, but hash difference is not a perceptual-distance claim.

## Structural macro admission

The rejected first mapping used low/high full-output sample residual as its
success criterion. The user heard seven nearly identical Triangle sounds. Raw
residual is therefore retained only as historical diagnostic evidence and is
not the structural admission rule.

Every TYPE position must pass all seven domain-specific landmarks:

| Macro | Evidence | Rejection floor |
| --- | --- | ---: |
| FORM | normalized 16-harmonic profile distance | 6 dB |
| WARP | normalized 16-harmonic profile distance | 6 dB |
| COUPLE | cycle decorrelation or normalized spectral-topology displacement | 0.25 |
| MOTION | rate ratio, conditional on at least 6 dB envelope depth at both ends | 200× |
| CHAOS | added nonrepeatability between successive modulation-cycle envelope contours | 0.08 |
| COLOR | harmonic-center ratio | 2× |
| SPACE | side/mid energy change | 12 dB |

COUPLE uses two valid manifestations because an already aperiodic source such
as the register machine can saturate a periodicity-only metric. It must either
lose at least 0.25 cycle correlation or move its normalized harmonic profile by
at least 6 dB; raw sample residual alone cannot pass it. CHAOS compares
windowed envelope contours so carrier phase does not masquerade as
irregularity.

Every prepared source and transition must also remain deterministic for a fixed
seed, resettable, finite, bounded, non-silent, and allocation-free. Construction
rejects invalid rates and frequencies. Low/middle/high notes retain peak, RMS,
DC, maximum-jump, stereo, mono, harmonic, hash, cyclic-envelope, and
high-rate-residual diagnostics. Severe sample-rate dependence remains an honest
production blocker even if the source is interesting to hear.

## Listening presentation

The ignored disposable batch contains exactly sixteen dry WAVs:

- eight separate neutral TYPE references;
- one uninterrupted live TYPE-dial reel through all eight positions; and
- seven plainly named `low, silence, high` reels on Saw.

Saw is the proof topology because the owner preferred it in the earlier source
comparison and because a known source makes weak macro travel difficult to
hide. Ordinary macro endpoints last two seconds. The demonstrated MOTION low
point lasts eleven seconds to expose one approximately 0.1 Hz cycle; its high
point is approximately 25 Hz. CHAOS endpoints last four seconds each to expose
repeatability versus held-cycle disruption.

All files use fixed source gain. There is no loudness matching, normalization,
ADSR, reverb, delay, chorus, compressor, limiter, master strip, direct input,
sampler, drums, another synth, JACK, ALSA, MIDI, or SHR-DAW in the render.

## Research boundary

The implementation is independently authored from mathematical and functional
claims in these sources. It copies no third-party source code, tables, presets,
samples, prose, or figures.

- Georg Essl, “Deforming the Oscillator: Iterative Phases Over Parametrizable
  Closed Paths,” DAFx-20in22, 2022,
  <https://www.dafx.de/paper-archive/2022/papers/DAFx20in22_paper_23.pdf>.
- Georg Essl, “Exploring the Sound of Chaotic Oscillators via Parameter
  Spaces,” DAFx-19, 2019, DAFx archive.
- Bill Verplank, Max Mathews, and Robert Shaw, “Scanned Synthesis,” ICMC 2000,
  <https://peabody.sapp.org/class/st2/read/ScannedSynthesis.PDF>.
- Sergio Luque, “The Stochastic Synthesis of Iannis Xenakis,” *Leonardo Music
  Journal* 19, 2009, pp. 77–84,
  <https://doi.org/10.1162/lmj.2009.19.77>.
- Ville-Matias Heikkilä, “Discovering Novel Computer Music Techniques by
  Exploring the Space of Short Computer Programs,” 2011,
  <https://arxiv.org/abs/1112.1368>, and Walt Kester, “MT-085: Fundamentals of
  Direct Digital Synthesis,” Analog Devices, 2008.

Publication copyright applies unless a source states otherwise. Only the
described mathematics and general topology inform the Rust implementation.

## Decision boundary

The user decides whether TYPE positions sound meaningfully different, whether
each macro produces a large useful transformation, and whether the collection
feels like one playable experimental instrument. Rejected audio is deleted
after its conclusion is recorded. Production integration needs explicit
selection plus a later native Raspberry Pi 1/2/4/8-voice callback gate.
