# Five-Family Listening Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Build a disposable monophonic offline comparison with one honest, measured representative of each approved sound family and stop for human listening before engine integration.

**Architecture:** A new `dsp::research` module owns five independent, fixed-state scalar sources behind a small `ResearchSource` enum that emits stereo frames without allocation. A separate `five-family-lab` binary renders and measures those sources only; it does not route them through `Engine`, presets, or stable macros. Deterministic reports and 15 loudness-matched WAVs live under ignored `artifacts/`.

**Tech Stack:** Stable scalar Rust 1.97.1, fixed-size arrays, existing `assert_no_alloc` and `hound`, deterministic offline analysis, Cargo audit/deny and AArch64 compile checks.

---

### Task 1: Research register and perceptual contracts

**Files:**
- Modify: `docs/RESEARCH.md`

- [x] Add a five-family primary-source register with author, title, publication details, direct URL, supported design claim, and licensing boundary. Use equations and independently authored topology only.
- [x] Define for every candidate what the listener should recognize, why its generator differs structurally from the other four, and what failure sounds like. Explicitly separate resonating comb delay from perceptual micro-delay.
- [x] Run `rg -n "Five-family source research|Perceptual hypothesis|Licensing" docs/RESEARCH.md`; verify all five entries exist before code and claim no result.

### Task 2: Research-source API and nonlinear PM candidate

**Files:**
- Create: `src/dsp/research.rs`
- Modify: `src/dsp/mod.rs`

- [x] Write tests constructing `ResearchSource::new(ResearchFamily::NonlinearPm, 48_000.0, 440.0)`, rejecting invalid input, and requiring deterministic reset, finite stereo samples bounded by 1.0, non-silence, allocation-free sampling, retained note energy, and broader sideband energy.
- [x] Run `cargo test dsp::research::tests::nonlinear_pm -- --nocapture`; verify RED because the module/API does not exist.
- [x] Implement `ResearchFamily`, `StereoFrame`, `ResearchError`, and `ResearchSource::{new,sample,reset}`. Use fixed phase accumulators, a static polynomial sine table, symmetric cubic modulator shaping, bounded PM index, and carrier lookup. Prepare rates outside sampling; use no per-sample transcendental setup.
- [x] Re-run the focused test; verify GREEN with no allocation.

### Task 3: Excited comb/resonant candidate

**Files:**
- Modify: `src/dsp/research.rs`

- [x] Write tests for deterministic seeded excitation, finite/bounded output, audible decay rather than silence/runaway, dominant pitch near the requested note, fixed delay bounds, reset identity, and allocation-free sampling.
- [x] Run `cargo test dsp::research::tests::excited_comb -- --nocapture`; verify RED because the variant is not implemented.
- [x] Implement three fixed-capacity delay lines with deterministic one-period noise excitation, one pitch-locked delay and two dispersed ratios, fractional reads, low-pass damping inside feedback, feedback below unity, and a bounded mono sum copied to stereo.
- [x] Re-run the focused test; verify GREEN.

### Task 4: Psychoacoustic micro-delay spatial-motion candidate

**Files:**
- Modify: `src/dsp/research.rs`

- [x] Write tests requiring declared sub-echo delay bounds, continuous movement, finite/bounded deterministic output, useful non-inverted stereo correlation, bounded side-to-mid energy, non-silent mono fold-down, small block-boundary jumps, and allocation-free sampling.
- [x] Run `cargo test dsp::research::tests::spatial_micro_delay -- --nocapture`; verify RED.
- [x] Feed a bounded band-limited buzz into four short unequal fractional-delay paths. Move complementary pairs with slow triangle modulators, use small opposing left/right gains plus a direct centered anchor, and smooth gains. Keep the identity controlled internal motion, not ping-pong echo or a wet/dry sweep.
- [x] Re-run the focused test; verify GREEN.

### Task 5: Spectral-traversal candidate

**Files:**
- Modify: `src/dsp/research.rs`

- [x] Write tests for deterministic reset, finite/bounded allocation-free output, suppression above 45% of sample rate, persistent fundamental pitch, and measurable spectral change between early/middle/late windows.
- [x] Run `cargo test dsp::research::tests::spectral_traversal -- --nocapture`; verify RED.
- [x] Implement a fixed recursive-sinusoid partial bank and four Moj Sint-authored amplitude frames. Traverse adjacent frames with a bounded slow triangle law, normalize frame energy, and taper partials approaching Nyquist.
- [x] Re-run the focused test; verify GREEN.

### Task 6: Small-register integer-machine swarm

**Files:**
- Modify: `src/dsp/research.rs`

- [x] Write tests for explicit 8/10/12/16-bit masks, wrapping add, bounded shifts, width-limited rotate, carry injection, deterministic reset, nonzero cycle length, no constant lockup, note-related transition rate, bounded float conversion, allocation-free sampling, and distinct hashes across widths.
- [x] Run `cargo test dsp::research::tests::integer_swarm -- --nocapture`; verify RED.
- [x] Implement a fixed array of seeded `u16` machines. Mask every transition to its width; use `wrapping_add`, width-limited rotate, explicit carry/borrow, fixed shifts, and an authored bit-selection mix. Derive increments from note and apply a prepared DC blocker after the one controlled float conversion.
- [x] Re-run the focused test; verify GREEN.

### Task 7: Shared measurements and alias/error evidence

**Files:**
- Create: `src/research.rs`
- Modify: `src/lib.rs`

- [x] Write failing tests for deterministic `ResearchMetrics`: peak, RMS, DC, fundamental/pitch retention, spectral distribution, hash, finiteness, correlation, side-to-mid, and mono energy. Add 8x-reference residual evidence for PM/integer candidates and high-note out-of-band/error checks for oscillatory candidates.
- [x] Run `cargo test research::tests -- --nocapture`; verify RED.
- [x] Implement offline `ResearchRenderSpec`, render/loudness helpers, spectral projections, hashing, spatial metrics, rapid movement/discontinuity probes, high-rate reference downsampling, and repeat/lockup reporting. Offline analysis may allocate; `ResearchSource::sample` may not.
- [x] Re-run focused tests; verify GREEN and distinct reports for all five families.

### Task 8: Disposable lab CLI and 15-file listening gate

**Files:**
- Create: `src/bin/five-family-lab.rs`
- Create: `tests/five_family_cli.rs`
- Generate only under: `artifacts/five-family-listening-gate/`

- [x] Write a failing CLI integration test invoking `five-family-lab render <directory>`. Require exactly 15 WAVs named by family and MIDI note 36/60/84, stereo 32-bit float at 48 kHz, plus deterministic manifests. Require the README to keep listening acceptance open and reject Pi claims.
- [x] Run `cargo test --test five_family_cli -- --nocapture`; verify RED because the binary is absent.
- [x] Implement the CLI to render one fixed representative per family for 1.5 seconds, add offline onset/end fades, loudness-match active windows without clipping, and write `manifest.tsv`, `spectral.tsv`, `spatial.tsv`, `alias-error.tsv`, `integer-cycles.tsv`, and separate volatile `workstation-cost.txt`.
- [x] Render twice into fresh temporary directories, exclude only volatile timing, and compare recursive hashes. Verify all WAVs and deterministic reports are byte-identical.

### Task 9: Documentation, handoff, and repository verification

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/superpowers/plans/2026-07-23-five-family-listening-gate.md`

- [x] Record only measured topology, hypotheses, measurements, limitations, filenames, deterministic evidence, and the open human verdict. State that no family, macro, Pi performance, polyphony, spatial translation, or sound quality is accepted.
- [x] Run `cargo fmt --check`, `cargo test --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo build --release`, `cargo audit`, `cargo deny check`, `cargo check --target aarch64-unknown-linux-gnu`, and `git diff --check`. Only the accepted `winnow` duplicate warning may remain.
- [x] Stop for human listening. Provide the ignored artifact directory and listening order. Do not select a winner, integrate into `Engine`, assign macros, preserve artifacts elsewhere, touch JACK/ALSA/SHR-DAW, push, or claim headphone/speaker/mono acceptance.

