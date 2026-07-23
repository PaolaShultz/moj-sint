# Composite Hot Audition, Octave, and Punch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve raw composite reconstructions while producing a fixed-gain
hot audition set, controlled D1/D2/D3 topology tests, D1 pedal scores, and one
measured per-source punch envelope per topology.

**Architecture:** Extend the isolated `composite_machine` research module with
declared gain policies, active-window measurement, pitch-controlled topology
rendering, D1 pedal scheduling, and a fixed-state research ADSR. Extend the lab
CLI with deterministic WAV organization and consolidated gain, octave, punch,
stereo, residual, similarity, allocation, and rejection reports.

**Tech Stack:** Stable scalar Rust, existing fixed hybrid voices, `hound`,
`assert_no_alloc`, deterministic TSV/Markdown output, Cargo audit/deny, and
AArch64 compile checking.

---

### Task 1: Lock raw references and define the hot contract

**Files:**
- Modify: `src/composite_machine.rs`

- [ ] Add failing tests for the previous raw reference hashes, the 4.25-8.00 s
  active-body window, explicit topology gains, 0.10-0.14 hot RMS, peak at most
  0.75, and no limiter/normalizer mode.
- [ ] Run the focused tests and observe failure for the absent gain API.
- [ ] Add `OutputGainPolicy`, fixed constants, active-window metrics, and pure
  multiplication after raw rendering.
- [ ] Run focused and library tests; confirm raw hashes remain unchanged.

### Task 2: Add pitch-controlled same-topology rendering

**Files:**
- Modify: `src/composite_machine.rs`

- [ ] Add failing tests for exact D1/D2/D3 frequencies, five heterogeneous
  held-note topologies, deterministic finite output, and one gain policy reused
  across all octaves.
- [ ] Add held-note source preparation and candidate topology rendering without
  per-octave normalization.
- [ ] Add D1 pedal score preparation that preserves existing musical layers and
  inserts or retunes one explicit low source before summing.
- [ ] Verify stereo, mono, low-band mid/side, projections, hashes, and residuals.

### Task 3: Add the research ADSR test-first

**Files:**
- Modify: `src/composite_machine.rs`

- [ ] Add failing tests for configuration rejection, exact stage timing,
  deterministic retrigger, continuous release, finite/bounded output, and
  allocation-free sampling.
- [ ] Implement prepared `ResearchAdsr` stages with explicit note events.
- [ ] Add failing placement tests proving envelope multiplication occurs per D1
  source before stereo matrixing/summing and references remain unchanged.
- [ ] Implement the punch pedal scheduler and automated bounded setting sweep.
- [ ] Retain one setting and report rejected settings without musical claims.

### Task 4: Expand deterministic reports and listening layout

**Files:**
- Modify: `src/bin/composite-machine-lab.rs`
- Modify: `tests/composite_machine_cli.rs`

- [ ] First require the new hot/reference/candidate, octave, bass, punch, mono,
  and raw files plus gain/octave/punch/allocation reports in the CLI test.
- [ ] Run the integration test and observe missing-output failure.
- [ ] Write the expanded WAV set and reports in the exact listening order.
- [ ] Add rejection rows for active RMS, peak, common octave gain, ADSR bounds,
  reference hashes, and forbidden final processors.
- [ ] Compare two quick generations byte-for-byte excluding workstation cost.

### Task 5: Render, inspect, and revise disposable output

**Files:**
- Generate only: `artifacts/composite-machine-lab/`

- [ ] Generate release output into two fresh temporary directories.
- [ ] Compare deterministic files and reconstruction hashes.
- [ ] Inspect all gain, octave, bass, punch, stereo, residual, similarity, and
  rejection rows.
- [ ] If a row fails, add a failing regression test and revise the gain
  structure/envelope; never add limiting or per-file maximization.
- [ ] Replace the ignored final batch only after evidence passes.

### Task 6: Durable documentation

**Files:**
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/RESEARCH.md`

- [ ] Record the old mismatch and perceptual bias, final fixed gain table,
  octave/bass findings, ADSR sweep/rejections/retained setting, numerical
  limitations, and exact human listening decision.
- [ ] State that digital level is not acoustic SPL or safe-volume evidence.

### Task 7: Verification, review, commits, and push

- [ ] Run formatting, all-target/all-feature tests, Clippy with warnings denied,
  release build, audit, deny, AArch64 check, deterministic release comparison,
  raw reconstruction regression comparison, and `git diff --check`.
- [ ] Critically inspect the generated reports and durable diff.
- [ ] Commit source/tests/docs only, confirm artifacts remain ignored, and push
  the verified commits to `origin/main`.
