# Moj Sint Struck Objects Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build three monophonic Moj Sint struck objects whose exact-source
impulses excite distinct resonant topologies with natural spectral decay.

**Architecture:** Add an isolated `struck_object` research module containing
prepared modal recurrences, source-derived excitation, topology state,
rendering, measurement, and rejection. Add a dedicated offline lab binary and
deterministic CLI test. Do not modify production `Engine` or preserve
experimental audio in Git.

**Tech Stack:** Scalar Rust, existing original hybrid source renderers,
prepared second-order modal recurrences, `assert_no_alloc`, `hound`, Cargo.

---

### Task 1: Declare three exact struck-object topologies

**Files:**
- Create: `src/struck_object.rs`
- Modify: `src/lib.rs`

- [x] Write a failing test requiring `StruckTopology::{CoupledWire,
  SpectralPlate, DualBridge}`, filenames, exact Cross/Spectral/Dual single-note
  exciters, `7/11/9 ms` excitation windows, `1600 ms` duration, and D2 base
  frequency.
- [x] Run `cargo test struck_object::tests::topologies_are_exact_and_distinct`
  and verify compilation fails because the module does not exist.
- [x] Implement the enum, constants, explicit mode-ratio/decay tables, topology
  labels, filenames, and source routing. Export the module from `src/lib.rs`.
- [x] Run the focused test and verify it passes.
- [x] Commit with `feat: define Moj Sint struck object topologies`.

### Task 2: Implement allocation-free modal excitation and natural decay

**Files:**
- Modify: `src/struck_object.rs`

- [x] Write failing tests requiring finite deterministic output, no sample-path
  allocation, exact 1600 ms duration, nonzero onset, exact final zero, and
  distinct hashes for all topologies.
- [x] Write a failing decay test requiring middle RMS below early RMS, late RMS
  below middle RMS, final reduction of at least 30 dB, and falling
  high-band/low-band ratio.
- [x] Run the focused tests and verify RED.
- [x] Implement prepared modal coefficients with the recurrence
  `y = impulse + 2*r*cos(w)*y1 - r*r*y2`; compute all trigonometric and damping
  coefficients during construction.
- [x] Derive excitation by differentiating and tapering the declared onset of
  the exact original source. Inject it into the modal states; never mix the dry
  source to output.
- [x] Implement Coupled Wire's paired eight-mode banks and bounded `0.0002`
  previous-velocity bridge exchange, Spectral Plate's eleven fixed modes, and
  Dual Bridge's two six-mode bodies with bounded `0.0003` previous-velocity
  exchange. Divide exchange by the peer-bank mode count.
- [x] Apply only the 0.5 ms boundary rise and final 100 ms boundary fade.
- [x] Run all `struck_object::tests` and verify GREEN.
- [x] Commit with `feat: render naturally decaying struck objects`.

### Task 3: Add shared gain and rejection evidence

**Files:**
- Modify: `src/struck_object.rs`

- [x] Write failing tests requiring one shared gain, whole-file RMS
  `-14..=-10 dBFS`, ceiling contact at most 1%, tonal D2 anchors, finite/DC/jump
  bounds, mono loss at most 1 dB, and no rejection reasons.
- [x] Retain a deliberately weak render test that must fail the total-RMS gate.
- [x] Run the tests and verify RED.
- [x] Implement preview, shared-gain selection, static ceiling, windowed
  early/middle/late RMS, spectral-decay evidence, existing tonal/noise metrics,
  stereo/mono metrics, and explicit rejection reasons.
- [x] Do not relax thresholds; reject a topology if one shared gain cannot pass.
- [x] Run the focused tests and verify GREEN.
- [x] Commit with `feat: reject defective struck object renders`.

### Task 4: Build the disposable listening lab

**Files:**
- Create: `src/bin/struck-object-lab.rs`
- Create: `tests/struck_object_cli.rs`

- [x] Write the CLI test first. Require exactly
  `01_coupled_wire.wav`, `02_spectral_plate.wav`, and `03_dual_bridge.wav`,
  stereo float 1600 ms audio, deterministic repeat generation, no piano/chord
  wording, and no WAV for rejected rows.
- [x] Run `cargo test --test struck_object_cli` and verify RED because the
  binary does not exist.
- [x] Implement `render` and `render-test`, passing-only WAV writing, README,
  manifest, mode table, metrics, decay evidence, rejections, hashes,
  generation summary, and workstation cost.
- [x] Run the CLI test, struck-object unit tests, and focused Clippy with
  warnings denied; verify GREEN.
- [x] Commit with `feat: present Moj Sint struck object comparisons`.

### Task 5: Generate, document, verify, and finish

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Generate ignored: `artifacts/moj-struck-objects/`

- [x] Generate two 48 kHz release batches in separate `mktemp -d` directories.
- [x] Compare every deterministic file, reject noise/weak/unstable output, and
  install only one batch containing exactly three passing WAVs.
- [x] Record the rejected piano-strike conclusion, topology contracts, exact
  metrics, deterministic aggregate hash, and open human gate.
- [x] Run `cargo fmt --check`, all-target tests, Clippy `-D warnings`, release
  build, `cargo audit`, `cargo deny check`, AArch64 compile, deterministic
  generation comparison, `git diff --check`, and artifact hygiene.
- [x] Commit documentation with `docs: record Moj Sint struck object gate`.
- [x] Use verification-before-completion and finishing-development-branch;
  fast-forward locally to `main`, rerun all-target tests, retain only
  `artifacts/moj-struck-objects/`, remove the worktree/branch, and do not push.
