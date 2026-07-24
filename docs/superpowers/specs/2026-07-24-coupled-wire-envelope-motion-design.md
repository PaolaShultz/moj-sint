# Coupled Wire Envelope and Motion Design

Date: 2026-07-24

## Status

Approved by the user on 2026-07-24.

## Musical Decision

The first Moj Sint struck-object candidate, Coupled Wire, passed human
listening as bright enough without becoming excessive. This experiment develops
that exact sound. It does not introduce a new exciter, pitch, modal ratio,
coupling graph, effect return, or replacement oscillator.

The user requested:

- the same Coupled Wire sound under different whole-sound envelopes;
- a `200 ms` attack;
- longer decay and a higher sustain level;
- a small swirling motion in either the high stereo image or a fast shallow
  vibrato; and
- direct execution without another broad sound-design search.

The selected motion is high-only stereo panning. It is less destructive than
vibrato because it leaves the pitch, low body, and mono sum unchanged.

## Architecture

The existing `StruckObject::new(CoupledWire, ...)` path remains the immutable
reference. A crate-private configured constructor may lengthen only the modal
loss times and total render duration. With the default duration and loss scale
of `1.0`, it must remain sample-identical to the accepted reference.

An isolated `coupled_wire_motion` module will:

1. render the exact Cross-derived Coupled Wire exciter and two-bank body;
2. multiply every declared modal decay time by one shared `5.0` loss scale;
3. split the completed stereo body into low and high components with prepared
   one-pole state at `320 Hz`;
4. add equal-and-opposite high-band pan motion to left and right, preserving
   the sample-exact mono sum before the master envelope; and
5. apply one master envelope to the complete stereo result.

The pan oscillator uses a prepared sine/cosine rotation. There is no
per-sample transcendental setup, allocation, lock, I/O, logging, formatting,
or nonlinear audio oscillator.

## Listening Files

The current accepted reference is included inside the new batch so the old
artifact directory can be removed without losing orientation.

| file | attack | decay | sustain | note-off | release | high pan |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `00_coupled_wire_reference.wav` | original | natural | none | none | final safety fade | none |
| `01_coupled_wire_warm_hold.wav` | 200 ms | 650 ms | 0.78 | 1900 ms | 1100 ms | none |
| `02_coupled_wire_slow_orbit.wav` | 200 ms | 900 ms | 0.84 | 2100 ms | 1200 ms | 2.6 Hz, depth 0.10 |
| `03_coupled_wire_fast_orbit.wav` | 200 ms | 500 ms | 0.74 | 1800 ms | 1500 ms | 6.2 Hz, depth 0.055 |

The reference remains `1600 ms`. Warm Hold is `3000 ms`; both Orbit files are
`3300 ms`. The envelope uses smooth polynomial stage curves and reaches exact
zero at the final frame. No note or layer enters after the common onset.

Only the high-pass residual above the prepared `320 Hz` split contributes to
the moving pan signal; the fundamental and low body are not intentionally
moved. The one-pole split is a control boundary, not an effect return: at zero
pan depth, recombining low and high reconstructs the input apart from
floating-point roundoff.

## Level and Rejection Contract

The exact reference retains its accepted gain of `4.43`. The three developed
versions use one new shared fixed gain selected from their unnormalized
renders. There is no per-file normalization, compressor, automatic gain,
soft saturation, reverb, delay, chorus, or dry parallel layer.

Every developed WAV must pass:

- finite output and allocation-free sampling;
- whole-file RMS from `-14` through `-10 dBFS`;
- static `-0.3 dBFS` ceiling contact no greater than `1%`;
- absolute DC no greater than `0.002`;
- bounded adjacent-sample jumps;
- tonal D2 inventory and conjunctive noise-like rejection;
- correlation above the existing mono-safe floor and mono loss no greater
  than `1 dB`;
- rising attack evidence across `0-60`, `80-140`, and `160-220 ms`;
- sustain energy above the low-RMS rejection floor before note-off;
- strictly falling release windows and exact final zero; and
- exact mono equality between motion-enabled and motion-disabled renders of
  the same envelope before presentation limiting.

If one shared gain cannot keep all three developed files inside the RMS and
ceiling bounds, the complete developed batch fails rather than normalizing a
file independently.

## Artifact Policy

The only active artifact directory at handoff will be:

`artifacts/coupled-wire-envelope-motion/`

It will contain the four WAVs above plus deterministic manifests, envelope and
motion specifications, metrics, rejection evidence, hashes, generation
summary, README, and volatile workstation-cost evidence. The prior
`artifacts/moj-struck-objects/` directory is removed only after two release
generations pass and the exact Coupled Wire reference has been reproduced
inside the new batch.

Nothing is routed into production `Engine`, presets, stable macros, JACK,
ALSA, or SHR-DAW. Human listening remains the musical gate.
