# Composite Machine Lab Implementation Plan

Status: implementation and three engineering iterations complete; final
verification/commit/push evidence is recorded in the handoff checkpoint.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reconstruct the delayed heterogeneous listening accident honestly, implement five distinct internally mixed composite hypotheses, reject engineering failures through two render iterations, and produce a disposable human listening batch.

**Architecture:** A new isolated `composite_machine` module owns fixed-state layer scheduling, prepared gains/matrices, two reconstruction references, and five hardwired composite topologies. A standalone lab binary performs offline rendering, ablation and high-rate comparisons, report generation, WAV output, and deterministic validation without touching production `Engine`.

**Tech Stack:** Stable scalar Rust, existing hybrid DSP, fixed arrays and prepared heap state, `assert_no_alloc`, `hound`, deterministic TSV/Markdown reports, Cargo audit/deny, and AArch64 compile checks.

**Predeclared rejection rules:** Follow `docs/superpowers/specs/2026-07-23-composite-machine-lab-design.md`: peak <= 0.8, absolute DC <= 5e-4, stereo correlation < 0.995, side/mid >= 0.005, mono RMS >= half stereo RMS, scheduled target retention within 36 dB, non-sparse layer contribution above -42 dB, final-candidate similarity below 0.985, active follower travel >= 3%, and risky-junction unintended difference products at least 30 dB below the weakest target.

---

### Task 1: Reconstruction inventory and fixed composite boundary

**Files:**
- Create: `src/composite_machine.rs`
- Modify: `src/lib.rs`

- [ ] Write tests for exact MIDI frequencies, synchronized inventory counts,
  the delayed schedule event timeline, invalid construction, deterministic
  reset, finite bounded samples, and allocation-free `CompositeMachine::sample`.
- [ ] Run `cargo test composite_machine::tests::boundary -- --nocapture` and
  verify failure because the module does not exist.
- [ ] Implement candidate/layer/score types, prepared original-condition gains,
  fixed progression state, sample-accurate offsets, fades, matrices, probes,
  reset, and frequency-inventory generation.
- [ ] Run the focused tests and keep the sample path allocation-free.

### Task 2: Distinct composite topologies and typed coupling

**Files:**
- Modify: `src/composite_machine.rs`

- [ ] Write failing tests for the reduced stack, role-separated machine,
  harmonic lattice, envelope-follower connection, and nonlinear braid.
- [ ] Require at least two families per candidate, distinct topology
  signatures, follower gain travel, bounded junction contribution, true stereo,
  and safe mono.
- [ ] Implement the five hardwired candidates with explicit roles, matrices,
  prepared headroom, follower state, and bounded multiplicative junction.
- [ ] Run focused tests and all library tests.

### Task 3: Segment metrics, ablation, similarity, and residual evidence

**Files:**
- Modify: `src/composite_machine.rs`

- [ ] Write failing tests for deterministic whole/segment metrics, three-band
  energy, spectral flux, target/spine projections, low-band stereo/mono,
  mute-one ablation, candidate similarity, nonlinear products, hashes, and
  high-rate residual.
- [ ] Implement offline-only analysis helpers; keep analysis allocation outside
  `CompositeMachine::sample`.
- [ ] Run focused tests and verify every predeclared engineering rejection is
  machine-checkable or explicitly reported.

### Task 4: Disposable lab CLI

**Files:**
- Create: `src/bin/composite-machine-lab.rs`
- Create: `tests/composite_machine_cli.rs`

- [ ] Write a failing quick-mode integration test requiring stereo candidates,
  mono diagnostics, README, manifest, layer schedule, frequency timeline,
  ablation, harmony, spectral, stereo, residual, topology, hash, rejection,
  and workstation-cost reports.
- [ ] Implement deterministic quick/full generation. Exclude only volatile
  workstation cost from byte comparison.
- [ ] Run the CLI test twice and compare deterministic outputs.

### Task 5: Two engineering iterations

**Files:**
- Modify source/tests as evidence requires.
- Modify: `docs/RESEARCH.md`

- [ ] Generate iteration A in a fresh temporary directory.
- [ ] Inspect every report; identify failed, inert, redundant, unsafe, or
  misleading candidates/layers.
- [ ] Record evidence-backed rejections, delete iteration A, and revise or
  replace failed mechanisms test-first.
- [ ] Generate iteration B fresh; rerun similarity, ablation, residual, stereo,
  mono, target, and deterministic comparisons.
- [ ] Continue until all final candidates are bounded and distinct without
  asserting musical success.

### Task 6: Final batch and durable documentation

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/MICRO_MACHINE_ROUTING.md`
- Generate only: `artifacts/composite-machine-lab/`

- [ ] Record the synchronized counterfactual inventory, delayed-launch
  uncertainty/estimate, implemented topologies, rejected revisions, causal
  ablations, typed-port implications, limitations, and open human verdict.
- [ ] Generate two fresh release batches and prove byte identity excluding cost.
- [ ] Generate the final ignored listening batch and inspect every report.

### Task 7: Full verification, review, commit, and push

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo test --all-targets --all-features`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo build --release`.
- [ ] Run `cargo audit`.
- [ ] Run `cargo deny check`.
- [ ] Run `cargo check --target aarch64-unknown-linux-gnu`.
- [ ] Repeat deterministic release generation comparison.
- [ ] Run `git diff --check`, inspect `git diff`, and confirm generated output
  remains ignored.
- [ ] Commit durable source/tests/docs and push the verified commit to
  `origin/main`.
