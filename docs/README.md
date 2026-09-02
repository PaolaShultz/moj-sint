# Moj Sint documentation

Source code, `Cargo.toml`, `rust-toolchain.toml`, preset parsers, and
`presets/cleared-presets.txt` define current behavior. This index separates
those contracts from research records, proposals, and dated session evidence.

## Current contracts

- [Architecture](ARCHITECTURE.md) covers engine, model, callback, and offline
  tool boundaries.
- [Live host contract](HOST_CONTRACT.md) defines process arguments, ALSA/JACK
  ports, event timing, overflow behavior, shutdown, and failure ownership.
- [Preset and control schema](PRESET_SCHEMA.md) defines schema 8, model
  identities, exact fields, MIDI CCs, physical positions, and migrations.
- [Portability and deployment](PORTABILITY.md) records toolchain, dependency,
  platform, and evidence limits.
- [Bass Matrix](BASS_MATRIX.md) records the current model design and automated
  evidence.
- [Dependencies, licensing, and public presets](../THIRD_PARTY.md) owns the
  public 21-preset boundary and dependency review.

SHR-DAW starts Moj Sint as an external managed process. Moj Sint owns synthesis,
preset validation, ALSA input, and stereo JACK output. SHR-DAW owns its exact
dependency pin, configuration, process lifecycle, routes, replacement recovery,
Project state, and private preset storage.

## Research and dated evidence

- [Research](RESEARCH.md) is the long-form source, experiment, rejection, and
  measurement record.
- [Bass impact research](BASS_IMPACT_RESEARCH.md) records the research basis
  and limits behind Bass Matrix.
- [Composite machine research](COMPOSITE_MACHINE_RESEARCH.md) preserves
  offline experiments and listening gates. It is not a live-engine contract.
- [Workspace handoff](HANDOFF.md) is a machine/session ledger. It includes
  historical states; source and the current contract documents above win when
  a dated entry conflicts with them.

## Proposed or incomplete work

- [Experimental direction](FUTURE_DIRECTION.md) describes unscheduled product
  and authoring ideas.
- [Micro-machine routing](MICRO_MACHINE_ROUTING.md) distinguishes the
  implemented Swarm Machine slice from deferred graph editing and general
  runtime proposals.

Files below `docs/superpowers/` are historical implementation plans. They are
useful provenance, but they do not define current behavior.
