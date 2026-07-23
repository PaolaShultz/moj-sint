# Compact Composite Power Lab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a deterministic intentionally hot listening batch of 8–12
genuinely different one-, two-, and three-mechanism composite experiments.

**Architecture:** Add an isolated fixed-state compact mechanism and score
module, then replace only the disposable composite-lab presentation. Preserve
the existing twelve-layer reconstruction implementation and hashes as
regression evidence.

**Tech Stack:** Scalar Rust, fixed sample-path state, prepared sine tables and
delay memory, `assert_no_alloc`, float WAV output, deterministic TSV/Markdown
reports, Cargo audit/deny, and AArch64 compile checking.

---

### Task 1: Compact mechanism and output boundary

**Files:**
- Create: `src/compact_composite.rs`
- Modify: `src/lib.rs`

- [ ] Add failing tests requiring 1–3 mechanisms, explicit policies, exact
  D1/D2/D3 frequencies, deterministic finite sampling, a `0.999` ceiling,
  sample-accurate events, and allocation-free sample paths.
- [ ] Run the focused test and confirm failure because the module is absent.
- [ ] Implement fixed mechanism states, score state, envelopes, stereo mix,
  and explicit output stages.
- [ ] Run focused tests and all library tests.

### Task 2: Measurement and causal evidence

**Files:**
- Modify: `src/compact_composite.rs`

- [ ] Add failing tests for active/event loudness, short-term perceptual
  proxy, tone/pitch retention, mono bass survival, event windows, clipping,
  return-to-zero, deterministic hashes, residual checks, and ablations.
- [ ] Run the tests and confirm the missing evidence APIs fail.
- [ ] Implement offline rendering, metrics, variants, mute ablations, and
  high-rate comparisons.
- [ ] Reject or simplify any topology with an inactive mechanism.

### Task 3: Disposable CLI and internal pool

**Files:**
- Replace: `src/bin/composite-machine-lab.rs`
- Modify: `tests/composite_machine_cli.rs`

- [ ] First require the new 8–12 primary layout, hot reference, mono
  diagnostics, exact settings, loudness/tone/pitch/event/ablation/stereo/
  residual/hash reports, and deterministic quick output.
- [ ] Run the integration test and observe the old 54-file layout fail.
- [ ] Implement quick/full rendering and report generation.
- [ ] Generate a larger temporary engineering pool and prune failed or
  redundant candidates without putting sweep grids in the listening batch.

### Task 4: Release exploration and critical review

**Generated only:**
- `artifacts/composite-machine-lab/`

- [ ] Generate two fresh release directories while measuring wall time.
- [ ] Compare every deterministic file and raw reconstruction hashes.
- [ ] Inspect every report for rejection rows, weak control regions, inert
  ablations, mono loss, accidental clipping, DC, jumps, tails, and misleading
  labels.
- [ ] Add failing regressions for discovered faults and rerun affected work.
- [ ] Replace the ignored final batch only after the reviewed run passes.

### Task 5: Durable documentation

**Files:**
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/RESEARCH.md`

- [ ] Record the three-mechanism maximum, hot/loudness-floor policy, retained
  topology/output settings, engineering rejections, event/mono/residual
  limitations, workstation generation cost, and exact human questions.
- [ ] Do not repeat the completed reconstruction investigation.

### Task 6: Verification, review, commits, and push

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo build --release`.
- [ ] Run `cargo audit`.
- [ ] Run `cargo deny check`.
- [ ] Run `cargo check --target aarch64-unknown-linux-gnu`.
- [ ] Repeat deterministic release comparison and reconstruction regression.
- [ ] Run `git diff --check`, inspect the complete diff and final reports,
  commit only durable work, and push the verified commit to `origin/main`.
