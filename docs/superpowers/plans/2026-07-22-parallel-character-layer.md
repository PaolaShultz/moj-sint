# Parallel Character Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one per-voice band-limited nonlinear character return over a bit-exact dry anchor and produce a nine-file measured listening gate.

**Architecture:** A focused DSP module owns a four-pole branch low-pass, cubic odd soft clip, selectable direct/ADAA/two-times research evaluation, and DC blocker. The engine keeps the existing `SHAPE`/`COLOR` base, retires the rejected selector from the voice render path, and uses smoothed `EDGE` for drive plus smoothed `COUPLE` for bounded return.

**Tech Stack:** Stable scalar Rust 1.97.1, existing `assert_no_alloc`, `hound`, deterministic offline analysis, Cargo audit and portability tools.

---

### Task 1: Character-layer primitive and antialias comparison

**Files:**
- Create: `src/dsp/character.rs`
- Modify: `src/dsp/mod.rs`

- [ ] Add tests for invalid sample rates, exact zero-return bypass, the cubic law and antiderivative at interior/boundary/exterior points, deterministic reset, finite bounds, DC decay, direct/ADAA/two-times outputs, and allocation-free sampling.
- [ ] Run `cargo test dsp::character::tests -- --nocapture`; verify RED because `dsp::character` does not exist.
- [ ] Implement `CharacterMethod::{Direct, Adaa1, Oversampled2x}` and `CharacterLayer::new`, `reset`, and `sample(dry, edge, couple)`. Prepare a 6 kHz four-pole low-pass coefficient and 10 Hz DC-block coefficient in `new`; map drive to `1 + 7*edge^2`, return to `0.35*couple`, and make `couple == 0` return `dry` exactly.
- [ ] Re-run the focused tests and verify GREEN, then run all library tests.

### Task 2: Per-voice engine routing

**Files:**
- Modify: `src/engine.rs`

- [ ] Replace the old `edge_and_couple_change_output` assertion with failing tests that `COUPLE=0` is sample-identical for every `EDGE`, adjacent `EDGE` and `COUPLE` travel changes the processed path, timed changes remain smoothed, rapid motion is finite/bounded, and `render_block` allocates nothing.
- [ ] Run `cargo test engine::tests -- --nocapture`; verify RED because the engine still routes the rejected harmonic selector.
- [ ] Remove `ThreePhaseBank` from `Voice`, add per-voice `CharacterLayer` using the evidence-selected method, reset it at note start, and process the colored base before envelope/velocity. Preserve the thirteen macro identities and document `EDGE`/`COUPLE` as candidate mappings only.
- [ ] Re-run focused and all-target tests and verify GREEN.

### Task 3: Measurements and method selection

**Files:**
- Modify: `src/analysis.rs`
- Modify: `src/bin/oscillator-lab.rs`
- Modify: `tests/offline_cli.rs`

- [ ] Add failing analysis and CLI tests for deterministic character metrics, harmonic levels, pitch retention, DC/peak/RMS, alias/error versus an eight-times offline reference, two-tone intermodulation reporting, and a `character <directory>` command.
- [ ] Run the focused tests and verify RED because the metrics and command are absent.
- [ ] Implement the non-real-time comparison for direct, ADAA1, and two-times evaluation on MIDI 36/60/84/96 at moderate and strong settings. Select the lowest-cost method within 3 dB of the best worst case that stays at or below -60 dB moderate and -50 dB strong; if none passes, reduce branch bandwidth or maximum drive and repeat.
- [ ] Record the selected method as an engine constant, re-run focused tests, and verify the declared method matches generated evidence.

### Task 4: Nine-file listening gate

**Files:**
- Modify: `src/bin/oscillator-lab.rs`
- Modify: `tests/offline_cli.rs`
- Create: `artifacts/parallel-character-milestone/README.md`
- Generate: `artifacts/parallel-character-milestone/*.wav`
- Generate: `artifacts/parallel-character-milestone/character-manifest.tsv`
- Generate: `artifacts/parallel-character-milestone/alias-comparison.tsv`
- Generate: `artifacts/parallel-character-milestone/intermodulation.tsv`
- Generate: `artifacts/parallel-character-milestone/workstation-cost.txt`

- [ ] Add a failing CLI test requiring exactly nine clearly named WAVs: dry/moderate/strong for notes 36/60/84, with one voice, `SHAPE=0.5`, `COLOR=0.5`, and loudness-matched active windows.
- [ ] Run the focused CLI test and verify RED before changing the generator.
- [ ] Generate the gate, keep volatile timing outside deterministic comparisons, render into two fresh temporary directories, and compare every WAV plus deterministic manifest byte-for-byte.

### Task 5: Research record and full verification

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/HANDOFF.md`

- [ ] Record all three compared graphs, selected per-voice topology, antialias method and measurements, DC/level/harmonic/pitch/intermodulation results, listening filenames, open user verdict, limitations, authorship, publication details, URL, and license boundary.
- [ ] State explicitly that scalar workstation evidence is not Raspberry Pi evidence and that no macro is accepted as useful yet.
- [ ] Run `cargo fmt --check`, `cargo test --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo build --release`, `cargo audit`, `cargo deny check`, and `cargo check --target aarch64-unknown-linux-gnu`.
- [ ] Perform a fresh two-directory deterministic render comparison, `git diff --check`, and `git status --short`; report the remaining listening decision without pushing.
