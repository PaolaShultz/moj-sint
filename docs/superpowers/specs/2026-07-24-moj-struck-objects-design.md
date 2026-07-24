# Moj Sint Struck Objects Design

Date: 2026-07-24

## Status

Approved for implementation by the user on 2026-07-24.

## Correction

This is not a piano imitation. The previous batch confused a request to borrow
the principle of a short strike and natural decay with a request for acoustic
piano timbre. It duplicated a source already present in the body, then applied
a broadband amplitude contour. Human listening rejected it as a piano with a
towel over its strings. Its generated artifact batch has been moved to Trash.

The replacement borrows only the useful physical abstraction:

```text
brief excitation -> energy-bearing resonant object -> natural loss
```

The resonant object, not an ADSR, creates the audible decay.

## Shared Boundaries

Every candidate is a single D2 pitch object, not a chord. It may use multiple
slightly inharmonic modes or same-note coupled resonators, but it may not
schedule another musical note. Each exciter is derived from the onset of one
exact original single-note source from the successful twelve-file pool. The
dry source is not mixed into the output; it injects energy into the new object.

All candidates:

- render for `1600 ms` at 48 kHz;
- use no sustain stage and no body ADSR;
- use only a `0.5 ms` boundary fade-in and final `100 ms` safety fade-out;
- prepare every coefficient and source buffer outside the sample path;
- use scalar, deterministic, allocation-free sampling;
- remain stereo through explicit unequal but mono-safe modal pickup matrices;
- use no delay/reverb/chorus effect, compressor, soft saturator, or
  per-candidate normalization; and
- feed one shared fixed presentation gain into the existing static
  `-0.3 dBFS` sparse-crest ceiling.

## Candidate 1: Coupled Wire

The first object uses the differentiated first `7 ms` of `CrossSingle` as a
force impulse. The impulse excites two same-note stiff-wire modal banks. Their
fundamentals differ by only `0.7 cents`, so they remain one D2 pitch rather
than a harmony. Eight modes per wire use progressively sharpened
inharmonic ratios. A bounded `0.012` per-sample bridge exchange transfers
energy between corresponding modes.

High modes decay first; low modes remain longer. The two pickups use different
modal sign and width patterns while preserving a centered low body. This
candidate tests lively beating and aftersound without chorus.

## Candidate 2: Spectral Plate

The second object uses the first `11 ms` of `SpectralSingle`, differentiated
and tapered, to excite eleven fixed modes. Its ratios begin with the D2
fundamental and then depart from a harmonic series into a sparse plate-like
pattern. The fundamental and octave anchors retain pitch; the remaining modes
produce a distinctly Moj Sint metallic field.

Each mode has its own loss coefficient. High, irregular modes disappear in
`90-280 ms`; the fundamental and low anchors persist for `700-1250 ms`.
Unequal left/right pickup weights create stereo without phase-inverting the
low anchor.

## Candidate 3: Dual Bridge

The third object uses the first `9 ms` of `DualSingle` as excitation for two
different six-mode bodies. One body emphasizes the D2 fundamental and odd
partials; the other uses octave and stretched upper modes. A one-sample
bounded bridge path transfers `0.018` of each body's previous summed velocity
into the other body.

This is not reverb or a feedback effect around a finished tone. The coupling
is inside the resonant object and is covered by stability and energy-decay
tests. It should create a round initial body followed by changing resonant
color.

## Natural Decay Contract

No candidate receives a perceptual amplitude envelope after excitation.
Modal damping must provide:

- onset energy in the first `50 ms`;
- strictly lower RMS in `350-700 ms` than in `50-200 ms`;
- strictly lower RMS in `1100-1450 ms` than in `350-700 ms`;
- at least `30 dB` reduction between the early and final windows;
- exact zero only through the shared final boundary fade; and
- no plateau lasting `100 ms` or more within the active decay.

The high-band/low-band energy ratio must fall across the first `700 ms`, so a
candidate cannot pass with one broadband volume envelope.

## Presentation and Rejection

Whole-file RMS is measured across all `1600 ms`. A candidate below
`-14 dBFS` receives no WAV. Shared gain selection must keep every candidate at
or below `-10 dBFS`; a candidate that cannot share the gain is rejected rather
than individually normalized.

A candidate also receives no WAV for:

- non-finite output or unstable modal energy;
- allocation in the sample path;
- excessive DC, adjacent-sample jump, or ceiling contact above `1%`;
- failed D2 fundamental and octave anchors;
- conjunctive noise-like classification;
- more than `1 dB` mono loss or inadequate stereo correlation;
- failed multi-window decay or spectral-decay rules; or
- a nonzero final frame.

Because the new renderer uses only linear prepared recurrences and source
differencing, it adds no nonlinear oscillator or waveshaper requiring a new
aliasing contract. Existing source aliasing limitations remain documented.

## Listening Batch

The only active artifact directory will be:

`artifacts/moj-struck-objects/`

It may contain at most:

1. `01_coupled_wire.wav`;
2. `02_spectral_plate.wav`; and
3. `03_dual_bridge.wav`.

Reports include the exact topology, source onset, mode ratios, decay
coefficients, shared gain, RMS windows, spectral-decay evidence, stereo/mono
evidence, rejection reasons, hashes, and workstation generation cost.

No old piano-strike artifact, piano wording, chord, solo, diagnostic WAV,
failed WAV, or low-RMS WAV may remain. Human listening is the musical
acceptance gate; nothing is routed into `Engine`, presets, or stable macros.
