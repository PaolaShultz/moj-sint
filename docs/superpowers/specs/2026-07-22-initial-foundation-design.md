# Moj Sint Initial Foundation Design

## Scope

This phase creates a portable, testable Rust foundation for Moj Sint. It does
not implement the live JACK/ALSA host, modify SHR-DAW, start audio services, or
claim Raspberry Pi performance. It produces one musically plain reference
oscillator so later oscillator research has a trustworthy baseline.

## Architecture

The library owns all deterministic synthesis behavior. `Engine` accepts
timestamped note and macro events and renders caller-owned stereo buffers with
preallocated voices. It has no file, JACK, ALSA, process, or wall-clock
dependencies. DSP primitives, envelopes, controls, preset validation, and
offline rendering are separate modules with narrow interfaces.

The binary initially exposes deterministic offline WAV rendering and preset
validation. A later `host` feature will adapt ALSA Sequencer input and a JACK
process callback to the same engine. The callback contract is documented now:
bounded event transfer; fixed storage; no allocation, locks, I/O, logging,
formatting, subprocesses, or panics in the callback.

## Reference Sound and Controls

The first sound is a quadrature-recurrence sine oscillator through a standard
ADSR. Frequency preparation happens on note events, keeping per-sample work to
bounded arithmetic without trigonometric calls.
The thirteen stable controls are represented by a closed enum and normalized
values. ADSR time controls use exponential mappings. Every control is smoothed;
the eight timbral macros and EVOLVE have stable identities but intentionally do
not pretend to be musically complete until later oscillator and routing work.
Presets reject unknown schema versions, non-finite/out-of-range normalized
values, invalid envelope times, and invalid voice counts.

## Preset and Offline Boundaries

`.mojsint` is a versioned TOML format with Moj Sint identity. Parsing is strict
and validation is explicit. Offline rendering takes a preset and a deterministic
note specification, constructs a fresh engine, and returns interleaved stereo
`f32`; WAV encoding is confined to the non-real-time offline module.

## Error Handling

Public configuration boundaries return typed errors. The render path uses
validated state and finite guards; invalid numeric results are replaced with
silence rather than propagated. The real-time API does not return allocation-
backed errors from its inner loop.

## Verification

Tests cover oscillator repeatability and finiteness, envelope transitions,
all thirteen control identities and ranges, strict preset validation, event
timing and voice bounds, stereo rendering, and byte-identical offline WAV
output. Native x86_64 formatting, tests, Clippy, and release builds are required.
AArch64 compilation is supporting evidence only; a native Pi build and measured
audio tests remain mandatory before performance claims.

## Deferred Work

The live JACK/ALSA adapters, exotic oscillators, factory presets, complete
macro routing/usefulness thresholds, Pi profiling, and SHR-DAW changes are
separate milestones. This keeps the first result small enough to verify while
preserving the accepted integration contract.
