# Moj Sint Initial Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a portable Rust synthesis core with strict presets and deterministic offline WAV rendering, ready for later JACK/ALSA adapters.

**Architecture:** A dependency-light library owns deterministic DSP and preallocated engine state; the CLI is a non-real-time adapter. Host contracts are documented without pulling system audio dependencies into the DSP core.

**Tech Stack:** Stable Rust, Cargo, serde, toml, hound, thiserror; rustfmt and Clippy.

---

### Task 1: Repository and crate skeleton

**Files:** Create `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`, `src/lib.rs`, `src/main.rs`, `README.md`.

- [ ] Initialize Git with `git init -b main`.
- [ ] Install rustup with the official minimal-profile installer and add rustfmt and Clippy.
- [ ] Add package metadata, a library, and the `moj-sint` binary.
- [ ] Run `cargo test`; expect the empty crate to compile.
- [ ] Commit the repository skeleton.

### Task 2: Stable control model

**Files:** Create `src/control.rs`; export it from `src/lib.rs`.

- [ ] Write tests proving exactly thirteen unique stable macro identifiers, normalized-value rejection outside `0..=1`, exponential ADSR time mapping, and smoothing convergence.
- [ ] Run `cargo test control`; expect unresolved control types.
- [ ] Implement `MacroId`, `Normalized`, time mapping, and allocation-free one-pole smoothing.
- [ ] Run `cargo test control`; expect all control tests to pass.
- [ ] Commit the control model.

### Task 3: Reference DSP and envelope

**Files:** Create `src/dsp/mod.rs`, `src/dsp/oscillator.rs`, `src/envelope.rs`; update `src/lib.rs`.

- [ ] Write oscillator tests for deterministic phase progression, bounded finite output, and reset.
- [ ] Run the focused tests; expect missing oscillator APIs.
- [ ] Implement a scalar `f32` sine phase accumulator and finite-value guard.
- [ ] Write ADSR tests for idle, attack, decay, sustain, release, and zero/invalid configuration rejection.
- [ ] Run the focused tests; expect missing envelope APIs.
- [ ] Implement a sample-rate-aware ADSR state machine with exponential control-time inputs.
- [ ] Run all DSP/envelope tests; expect them to pass.
- [ ] Commit the reference DSP.

### Task 4: Strict versioned presets

**Files:** Create `src/preset.rs`, `presets/reference.mojsint`; update `src/lib.rs`.

- [ ] Write tests that parse a complete version-1 preset and reject unknown fields, versions, non-finite/out-of-range macros, impossible voice counts, and invalid envelope values.
- [ ] Run `cargo test preset`; expect missing preset APIs.
- [ ] Implement serde-backed strict TOML parsing, validation, typed errors, and defaults only where the schema explicitly permits them.
- [ ] Run `cargo test preset`; expect all preset tests to pass.
- [ ] Commit the preset boundary and reference preset.

### Task 5: Preallocated event-driven engine

**Files:** Create `src/engine.rs`; update `src/lib.rs`.

- [ ] Write tests for sample-offset note-on/off, fixed voice capacity/stealing, stereo equality for the mono reference voice, finite output, and allocation-free rendering after construction.
- [ ] Run `cargo test engine`; expect missing engine APIs.
- [ ] Implement fixed-capacity voices, a fixed-capacity event block, MIDI note conversion, block rendering into caller-owned slices, and control smoothing.
- [ ] Run `cargo test engine`; expect all engine tests to pass.
- [ ] Commit the synthesis engine.

### Task 6: Deterministic offline renderer and CLI

**Files:** Create `src/offline.rs`, `tests/offline_cli.rs`; update `src/lib.rs`, `src/main.rs`, `Cargo.toml`.

- [ ] Write library and CLI tests for repeatable samples, valid stereo WAV metadata, preset validation, and clear invalid-input failures.
- [ ] Run focused tests; expect missing renderer/CLI behavior.
- [ ] Implement note rendering, WAV writing, `render` and `validate` commands without adding a general CLI framework.
- [ ] Run focused tests twice and compare WAV SHA-256 hashes; expect identical hashes.
- [ ] Commit offline rendering.

### Task 7: Integration, portability, and research documentation

**Files:** Create `docs/ARCHITECTURE.md`, `docs/HOST_CONTRACT.md`, `docs/PORTABILITY.md`, `docs/RESEARCH.md`, `AGENTS.md`; update `README.md`, `docs/HANDOFF.md`.

- [ ] Recheck actively changing SHR-DAW contracts at its current `main` revision and record the revision.
- [ ] Document the future JACK/ALSA adapter, callback prohibitions, exact two-port/client behavior, bounded MIDI handoff, shutdown, and fourth-backend changes.
- [ ] Document native Pi verification, AArch64 supporting checks, dependency commands, and prohibited performance claims.
- [ ] Curate authoritative/primary DSP, JACK, ALSA, Rust tooling, licensing, Pi, and listening-test sources with authorship and why each matters.
- [ ] Record evaluated skills/MCPs/tools and only install maintained tools needed now.
- [ ] Create concise repository-specific agent instructions.
- [ ] Commit documentation.

### Task 8: Final verification

**Files:** Modify any file producing verification failures; update `docs/HANDOFF.md` with exact evidence.

- [ ] Run `cargo fmt --check`; expect success.
- [ ] Run `cargo test --all-targets --all-features`; expect success.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`; expect success.
- [ ] Run `cargo build --release`; expect success.
- [ ] Render the reference note twice and verify byte-identical WAV hashes.
- [ ] Attempt an AArch64 compile check if the Rust target can be installed without system packages; clearly distinguish it from native Pi evidence.
- [ ] Inspect `git status`, `git diff --check`, dependency licenses, and remaining system dependencies.
- [ ] Commit the verified checkpoint and report the next smallest musical milestone.
