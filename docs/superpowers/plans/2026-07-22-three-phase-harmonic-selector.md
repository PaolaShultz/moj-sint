# Three-Phase Harmonic Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one per-voice shared-phase harmonic selector controlled by smoothed `EDGE` and `COUPLE`, with deterministic measurement and mono listening evidence.

**Architecture:** A focused DSP module derives 0/120/240-degree sine taps from one recursive phase state and uses their cubic sum to isolate a normalized third harmonic. The engine blends fundamental-to-third with `EDGE`, then existing oscillator-to-selector with `COUPLE`; offline tooling measures the topology and renders the approved evidence matrix.

**Tech Stack:** Stable scalar Rust 1.97.1, existing `assert_no_alloc`, `hound`, Cargo verification tools, project-owned deterministic analysis.

**Listening outcome:** The user rejected all generated conditions as too close
to pure sine material. The generated repository artifact corpus was removed;
the lab command now produces evidence only in temporary test or explicitly
requested output directories. This plan records a negative experiment, not an
accepted factory sound.

---

### Task 1: Shared-phase DSP primitive

**Files:**
- Create: `src/dsp/harmonic_selector.rs`
- Modify: `src/dsp/mod.rs`

- [x] Write tests for 120-degree tap algebra, equal-linear-tap cancellation, normalized third-harmonic selection, high-note fallback before Nyquist, deterministic reset, finite bounds, invalid sample rates, and allocation-free sampling.
- [x] Run `cargo test dsp::harmonic_selector::tests -- --nocapture` and confirm RED because the module/API is missing.
- [x] Implement a per-voice recursive sine/cosine state whose frequency setup prepares rotation coefficients, whose sample method derives all taps algebraically, and whose cubic selector uses only bounded scalar arithmetic.
- [x] Re-run the focused tests and confirm GREEN.

### Task 2: Smoothed engine routing

**Files:**
- Modify: `src/engine.rs`

- [x] Add failing tests that `EDGE` and `COUPLE` change adjacent travel segments at the reference operating point, are smoothed after timed events, stay finite/bounded during rapid movement, and allocate nothing in `render_block`.
- [x] Run `cargo test engine::tests -- --nocapture` and confirm RED because these macro routes are inert.
- [x] Add one selector and two 10 ms smoothers to each voice, prepare/reset them on note start, and blend selector and existing paths with bounded algebraic compensation.
- [x] Re-run focused and library tests and confirm GREEN without weakening existing `SHAPE`/`COLOR` checks.

### Task 3: System measurement and listening artifacts

**Files:**
- Modify: `src/analysis.rs`
- Modify: `src/bin/oscillator-lab.rs`
- Modify: `tests/offline_cli.rs`
- Create: `artifacts/harmonic-selector-milestone/*`

- [x] Add a failing CLI test for a deterministic `system` output containing 27 clearly named mono-condition WAVs, an A/B baseline, and a metrics manifest.
- [x] Run `cargo test --test offline_cli oscillator_lab_writes_harmonic_selector_evidence -- --nocapture` and confirm RED because the command is missing.
- [x] Implement offline harmonic projection, a high-note alias matrix, non-harmonic residual, peak/RMS/DC/hash metrics, pitch-retention fields, loudness matching, and release workstation timing outside the render path.
- [x] Generate the artifact directory twice and prove byte-identical WAV/manifest output; keep volatile timing in a separate report.

### Task 4: Documentation and repository verification

**Files:**
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/HANDOFF.md`
- Create: `artifacts/harmonic-selector-milestone/README.md`

- [x] Record the selected graph, alternatives, macro semantics, measurements, workstation-only cost, source/licensing boundary, and open human-listening gate.
- [x] Run `cargo fmt --check`, `cargo test --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo build --release`, `cargo audit`, `cargo deny check`, and `cargo check --target aarch64-unknown-linux-gnu`.
- [x] Render two fresh artifact matrices, compare deterministic files byte-for-byte, then run `git diff --check` and inspect `git status --short`.
