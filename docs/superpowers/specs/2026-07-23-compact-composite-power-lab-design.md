# Compact Composite Power Lab Design

Date: 2026-07-23

## Goal and boundary

Replace the disposable 54-file audition presentation with a broad but
comprehensible experiment in which every retained sound uses one, two, or
three simultaneous source mechanisms. Preserve the old synchronized and
delayed reconstruction code and hashes as durable regression evidence, but use
only the hot delayed reference as listening orientation.

The work stays in the isolated composite research lab. It does not change
`Engine`, production presets, stable macros, JACK, ALSA, SHR-DAW, or the
existing hybrid voices.

## Architecture

Add a focused `compact_composite` module rather than further enlarging the
existing reconstruction module. A `CompactMachine` owns at most three fixed
mechanism states, a deterministic sample-accurate score, prepared source and
stereo gains, and an explicit output stage.

The mechanism vocabulary is deliberately small and heterogeneous:

- table oscillator with a stable fundamental;
- bounded nonlinear phase-interaction oscillator;
- authored recursive spectral bank;
- fixed-width register source;
- deterministic noise transient;
- damped resonant body; and
- excited feedback comb.

Each retained topology declares its exact mechanism list, source gains, tone
range, pitch/event role, envelopes, drive method, drive amount, output gain,
and digital ceiling. Construction may allocate tables and delay memory.
`CompactMachine::sample` may not allocate/deallocate, lock, perform I/O, log,
format, panic, spawn, or perform transcendental setup.

## Experiment set

Build an internal pool larger than the final set and retain approximately ten
different experiments spanning:

1. sustained low pressure;
2. nonlinear playable mid voice;
3. evolving spectral/register body;
4. D1 pedal;
5. D2 bass machine;
6. short clipped bass thump;
7. resonant bass thump;
8. synthetic pitch-drop kick;
9. struck/excited comb object; and
10. a short musical-context relay.

The context render is a separate topology/score, not a normalized montage of
other WAVs. Parameter sweeps remain reports only.

## Output and loudness policy

Every topology uses one declared gain/drive policy across D1/D2/D3 and low,
middle, and high tone positions. There is no per-file peak normalization,
automatic make-up gain, or analysis-dependent limiter.

The output stage is one of:

- explicit linear gain;
- bounded cubic saturation;
- rational saturation; or
- declared hard clipping.

An optional final ceiling is part of that named output topology and is never
hidden. Written samples must remain finite and at or below `0.999`. Extremely
hot candidates are valid. Digital level is not acoustic SPL; the README must
warn listeners to start with playback volume low.

The hot delayed reconstruction supplies the loudness floor. Sustained and
musical candidates compare a common active window; events compare a common
event window. Reports include active/event RMS, a documented short-term
perceptual loudness proxy, low-band energy, crest, envelope-energy windows,
peak, clipping proportion, and hashes. Automated measurements reject obvious
loudness collapse but cannot establish perceived power or musical value.

## Evidence and rejection rules

Before implementation, tests must require:

- one to three simultaneous mechanisms;
- deterministic, finite, allocation-free sampling;
- explicit source gain, drive, saturation/clipping, output gain, and envelope
  declarations;
- written peak no greater than `0.999`;
- no automatic per-file normalization;
- one gain policy across pitch and tone variants;
- deterministic sample-accurate event/envelope scheduling;
- retained tone travel with no RMS or loudness-proxy collapse beyond 4 dB;
- bass mono fold loss no greater than 1.5 dB and no severe fundamental loss;
- conservative high-rate residual evidence for nonlinear paths;
- deterministic hashes; and
- one mute ablation per retained mechanism with an active structural
  contribution.

Event reports add first 10/25/50/100/250 ms RMS and peak, onset/body energy,
low-band transient energy, fundamental/pitch trajectory, decay/tail, clipped
sample proportion, maximum jump, DC, return-to-zero, and mono behavior.
Stereo reports add correlation, side/mid, low-frequency side/mid, mono fold,
and cancellation.

## Disposable workflow and human gate

Generate into two fresh temporary directories. Compare deterministic output
byte-for-byte except the wall-clock report, review every rejection and
measurement row, revise failures test-first, and only then replace
`artifacts/composite-machine-lab/`.

The final primary section contains 8–12 WAVs. The hot reference, mono
diagnostics, and raw engineering reports do not count. No generated WAV,
report, sweep, parameter dump, or manifest is committed. Human listening
decides whether any experiment is powerful, distinguishable, special, or
worth preserving.
