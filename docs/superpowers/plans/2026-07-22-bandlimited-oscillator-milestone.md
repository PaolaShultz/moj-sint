# Bandlimited Oscillator Milestone Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compare fair scalar PolyBLEP and generic integrated-wavetable oscillators, select one from measured evidence, and route it through smoothed `SHAPE` and `COLOR` macros with deterministic listening renders.

**Architecture:** Both candidates share a normalized phase accumulator and the same saw-to-square target family. A non-real-time analysis module compares them with a band-limited additive reference and reports residual alias/error energy, peak, RMS, DC, determinism, and finiteness; the lower-error candidate that also satisfies bounded real-time behavior becomes the engine oscillator. Per-voice smoothers drive continuous waveform morphing and a bounded one-pole harmonic-color stage without allocation, I/O, locks, or per-sample transcendental setup.

**Tech Stack:** Stable scalar Rust, existing `assert_no_alloc` allocator guard, existing `hound` WAV writer, deterministic radix-2 spectral analysis implemented in project test/offline code, Cargo quality and portability checks.

---

### Task 1: Candidate oscillator contract

**Files:**
- Modify: `src/dsp/oscillator.rs`
- Test: `src/dsp/oscillator.rs`

- [ ] **Step 1: Write failing oscillator contract tests**

Add tests that construct both candidate methods at 48 kHz, set representative MIDI-note frequencies, render saw/square/midpoint shapes, and assert exact repeatability, reset repeatability, finite samples, bounded peaks, and correct invalid-sample-rate rejection. Add a phase-increment test that proves frequency changes are prepared outside `sample`.

- [ ] **Step 2: Run the focused test and verify RED**

Run: `cargo test dsp::oscillator::tests -- --nocapture`

Expected: compilation fails because `BandlimitedOscillator`, `OscillatorMethod`, or the new shape-aware sample API does not exist.

- [ ] **Step 3: Implement the two minimal scalar candidates**

Implement:

```rust
pub enum OscillatorMethod { PolyBlep, IntegratedWavetable }

pub struct BandlimitedOscillator {
    method: OscillatorMethod,
    phase: f32,
    phase_increment: f32,
    previous_integrated_saw: f32,
    previous_integrated_square: f32,
}
```

Use the second-order integrated-linear-interpolator PolyBLEP residual around discontinuities for the PolyBLEP branch. For the integrated-wavetable branch, use compile-time fixed zero-mean integrated saw and square tables, linear lookup, one-sample differentiation, and phase-increment normalization. Keep tables shared, methods independently implemented, and all sample work bounded and allocation-free.

- [ ] **Step 4: Run focused and library tests and verify GREEN**

Run: `cargo test dsp::oscillator::tests -- --nocapture && cargo test --lib`

Expected: all oscillator and existing library tests pass.

### Task 2: Evidence-producing comparison

**Files:**
- Create: `src/analysis.rs`
- Modify: `src/lib.rs`
- Create: `src/bin/oscillator-lab.rs`
- Test: `src/analysis.rs`

- [ ] **Step 1: Write failing analysis tests**

Define a `WaveformMetrics` result with residual alias/error decibels relative to a finite band-limited Fourier reference, peak, RMS, DC, deterministic hash, and finite status. Test a known sine (near-zero residual against its reference), deterministic repeated candidate measurements, and a deliberately naive discontinuous waveform that measures worse than at least one corrected candidate.

- [ ] **Step 2: Run analysis tests and verify RED**

Run: `cargo test analysis::tests -- --nocapture`

Expected: compilation fails because the analysis API does not exist.

- [ ] **Step 3: Implement deterministic offline analysis and lab output**

Implement measurement outside the render callback. Use coherent analysis intervals, skip oscillator warm-up, remove reference/output DC, fit one bounded scalar gain before residual calculation, and label the result `alias_error_db` because it is a conservative sum of aliasing plus amplitude/phase deviation from the ideal band-limited target. The lab binary prints machine-readable TSV and Markdown rows for both methods over several notes and shapes.

- [ ] **Step 4: Verify the analysis tests and run the comparison**

Run: `cargo test analysis::tests -- --nocapture`

Run: `cargo run --release --bin oscillator-lab -- compare artifacts/oscillator-milestone`

Expected: tests pass; the command writes a comparison report with finite metrics for each method/note/shape and exits successfully.

- [ ] **Step 5: Select the winner from recorded evidence**

Choose the method with lower aggregate and worst-case `alias_error_db`, provided it stays deterministic, finite, bounded, and allocation-free. If results split, prefer the method with the lower worst case, then lower state/table cost; record the rule and raw rows rather than making a sound-quality claim.

### Task 3: Smoothed SHAPE and COLOR engine routing

**Files:**
- Modify: `src/engine.rs`
- Modify: `src/control.rs`
- Modify: `src/offline.rs`
- Test: `src/engine.rs`
- Test: `src/offline.rs`

- [ ] **Step 1: Write failing macro-route tests**

Add `Event::SetMacro { id: MacroId, value: Normalized }`. Test that preset `SHAPE`/`COLOR` values initialize every voice, changes at timed sample offsets are smoothed rather than stepped, low/mid/high settings produce measurable waveform/RMS/spectral differences across most of travel, rapid changes remain finite and bounded, and engine renders remain allocation-free.

- [ ] **Step 2: Run focused tests and verify RED**

Run: `cargo test engine::tests offline::tests -- --nocapture`

Expected: compilation fails because macro events and oscillator routing do not exist.

- [ ] **Step 3: Implement minimal macro routing**

Give each voice the selected oscillator, 10 ms `SHAPE` and `COLOR` smoothers initialized from the preset, and a scalar color-filter state. `SHAPE` continuously crossfades the corrected saw/square family over normalized travel. `COLOR` moves between a stable harmonic-dark state and the direct bright oscillator using a bounded algebraic coefficient. Apply only capped polynomial level compensation; retain the preset output gain and final finite guard.

- [ ] **Step 4: Verify GREEN and allocation behavior**

Run: `cargo test engine::tests offline::tests -- --nocapture`

Run: `cargo test --all-targets --all-features`

Expected: all tests pass, including `assert_no_alloc` around note rendering and rapid macro events.

### Task 4: Listening artifacts and measurement documentation

**Files:**
- Modify: `src/bin/oscillator-lab.rs`
- Create: `artifacts/oscillator-milestone/README.md`
- Create: `artifacts/oscillator-milestone/*.wav`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/HANDOFF.md`
- Test: `tests/offline_cli.rs`

- [ ] **Step 1: Write failing lab/render CLI tests**

Test stable argument parsing and clearly named output paths for a matrix covering at least three notes and low/mid/high `SHAPE` and `COLOR` positions. Assert stereo 32-bit float WAV metadata, finite samples, non-silence, and deterministic byte output.

- [ ] **Step 2: Run the CLI tests and verify RED**

Run: `cargo test --test offline_cli -- --nocapture`

Expected: the new lab render mode test fails until implemented.

- [ ] **Step 3: Implement and generate the listening matrix**

Run: `cargo run --release --bin oscillator-lab -- render artifacts/oscillator-milestone`

Expected: clearly named WAVs encode note and macro positions, plus a manifest containing SHA-256 hashes and measured peak/RMS/DC values.

- [ ] **Step 4: Document evidence and limitations**

Record method definitions, fair-comparison conditions, raw and aggregate metrics, selected implementation, allocation evidence, deterministic hashes, source authors/publication/URLs/licensing notes, and the explicit limits: workstation measurements are not Pi evidence and automated macro differences are not human musical-usefulness acceptance.

### Task 5: Full verification and handoff

**Files:**
- Modify only as required by verification findings

- [ ] **Step 1: Run formatting and all tests**

Run: `cargo fmt --check`

Run: `cargo test --all-targets --all-features`

- [ ] **Step 2: Run lint and native release build**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Run: `cargo build --release`

- [ ] **Step 3: Run dependency policy checks**

Run: `cargo audit`

Run: `cargo deny check`

- [ ] **Step 4: Run AArch64 compile evidence**

Run: `cargo check --target aarch64-unknown-linux-gnu`

Expected: compile succeeds; document that this is not a native Raspberry Pi build or performance measurement.

- [ ] **Step 5: Re-render determinism evidence**

Generate the same listening matrix into two temporary directories and compare recursive SHA-256 manifests byte-for-byte.

- [ ] **Step 6: Inspect repository state and requirements**

Run: `git diff --check`

Run: `git status --short`

Re-read this plan and the user request, verify every requested artifact/evidence item, and report any unresolved limitation without claiming human listening acceptance or Pi performance.
