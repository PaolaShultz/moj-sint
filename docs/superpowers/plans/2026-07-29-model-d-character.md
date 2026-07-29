# Model D Character Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify an isolated, monophonic, circuit-informed Model D character voice with a deterministic seven-file listening and causal-ablation lab.

**Architecture:** Three prepared bandlimited VCOs feed a bounded nonlinear mixer, a four-times-oversampled four-stage nonlinear ladder, and an ADS-style loudness contour; a second contour moves the ladder cutoff and a bounded output-feedback path returns before the filter. Production `Engine` and presets remain untouched; offline rendering, measurements, reports, and generated WAVs live behind a research module and CLI.

**Tech Stack:** Scalar Rust 2024, existing `hound`, `thiserror`, and `assert_no_alloc`; standard Cargo format/test/Clippy/build/audit/deny/AArch64 checks.

**Execution record:** Tasks 1–6 are implemented in repository history. Checked
boxes record delivered code, tests, or current verification, not a
retrospective claim that every intermediate command produced the originally
predicted output. In particular, historical RED command output was not
preserved, so those six RED-only boxes remain unchecked and explicitly noted.
The alias acceptance method changed during implementation as recorded in Task
5 and the design.

---

## File map

- Create `src/model_d/vco.rs`: prepared VCO waveform, pitch, drift, mismatch, and tests.
- Create `src/model_d/mixer.rs`: four-input linear/nonlinear mixer and tests.
- Create `src/model_d/ladder.rs`: oversampled nonlinear ladder, cutoff table, resonance, and tests.
- Create `src/model_d/contour.rs`: Model D ADS contour wrapper and tests.
- Create `src/model_d/voice.rs`: patch definitions and complete prepared voice.
- Create `src/model_d.rs`: research-model public API.
- Create `src/model_d_lab.rs`: authored scores, rendering, metrics, alias measurements, and report data.
- Create `src/bin/model-d-lab.rs`: deterministic artifact writer.
- Create `tests/model_d_cli.rs`: complete CLI and file-contract regression.
- Modify `src/lib.rs`: expose the two isolated research modules.
- Modify `docs/RESEARCH.md`: record sources, implementation interpretation, and evidence boundary.
- Modify `docs/HANDOFF.md`: record implemented scope, verification, artifact path, and listening gate.
- Modify `/home/shome/Documents/knowledge/Moj Sint/01 Current State.md`: route future sessions to the new pending listening gate.

### Task 1: Prepared VCO bank

**Files:**
- Create: `src/model_d/vco.rs`
- Create: `src/model_d.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add VCO contract tests**

Add tests that construct three `ModelDVco` values with `VcoConfig`, reject invalid sample rates/configuration, verify deterministic reset, bound static offset and drift to the specification, verify MIDI 36/60/84 pitch at 44.1/48/96 kHz, and wrap `sample()` in `assert_no_alloc`.

The public API fixed by the tests is:

```rust
pub enum ModelDWaveform { Triangle, Saw, Rectangle, WidePulse, NarrowPulse }
pub struct VcoConfig {
    pub semitone_offset: i8,
    pub cents_offset: f32,
    pub drift_cents: f32,
    pub drift_hz: f32,
    pub asymmetry: f32,
    pub level_offset: f32,
    pub reset_phase: f32,
    pub waveform: ModelDWaveform,
}
pub struct ModelDVco { /* fixed scalar state */ }
impl ModelDVco {
    pub fn new(sample_rate: f32, config: VcoConfig) -> Result<Self, ModelDError>;
    pub fn set_note(&mut self, note: u8);
    pub fn sample(&mut self) -> f32;
    pub fn reset(&mut self);
}
```

- [ ] **Step 2: Run the focused test and verify RED** — historical command
  output was not preserved; no retrospective RED claim is made.

Run: `cargo test model_d::vco -- --nocapture`

Expected: compile failure because `model_d::vco` and its types do not exist.

- [x] **Step 3: Implement the minimum prepared VCO**

Implement an independent phase accumulator, PolyBLEP saw/pulse edges,
continuous triangle, recurrence-based drift oscillator, prepared cents ratio,
bounded asymmetry, deterministic phase/reset state, and finite guards. Keep
`powf` and `sin_cos` in construction or `set_note`; the sample path performs
fixed arithmetic only.

- [x] **Step 4: Run VCO tests and the existing oscillator tests**

Run:

```bash
cargo test model_d::vco -- --nocapture
cargo test dsp::oscillator -- --nocapture
```

Expected: all selected tests pass.

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/model_d.rs src/model_d/vco.rs
git commit -m "feat: add prepared Model D oscillators"
```

### Task 2: Contour and nonlinear mixer

**Files:**
- Create: `src/model_d/contour.rs`
- Create: `src/model_d/mixer.rs`
- Modify: `src/model_d.rs`

- [x] **Step 1: Add contour and mixer contract tests**

Contour tests require attack, decay, sustain, release using the same decay
duration, exact idle zero, deterministic restart, invalid-config rejection,
and allocation-free `advance()`. Mixer tests require identical linear
summation, monotonic nonlinear transfer through the declared input range,
compression at high summed level, added odd-harmonic energy, finite output,
reset determinism, and allocation-free `sample()`.

The APIs are:

```rust
pub struct ContourConfig {
    pub attack_seconds: f32,
    pub decay_seconds: f32,
    pub sustain_level: f32,
}
pub struct ModelDContour { /* ADS state */ }
pub enum MixerMode { Linear, Nonlinear }
pub struct MixerConfig {
    pub source_levels: [f32; 3],
    pub feedback_level: f32,
    pub drive: f32,
    pub mode: MixerMode,
}
pub struct ModelDMixer { /* fixed config */ }
```

- [ ] **Step 2: Run focused tests and verify RED** — historical command output
  was not preserved; no retrospective RED claim is made.

Run:

```bash
cargo test model_d::contour -- --nocapture
cargo test model_d::mixer -- --nocapture
```

Expected: compile failure because the modules do not exist.

- [x] **Step 3: Implement contour and mixer**

Implement the contour as a fixed state machine. Implement the mixer with an
odd cubic soft clip inside `[-1, 1]`, saturated shoulders outside that interval,
explicit drive compensation, and a separate exact linear mode. Reject
non-finite or out-of-range preparation values.

- [x] **Step 4: Run focused and envelope regression tests**

Run:

```bash
cargo test model_d::contour -- --nocapture
cargo test model_d::mixer -- --nocapture
cargo test envelope::tests -- --nocapture
```

Expected: all selected tests pass.

- [x] **Step 5: Commit**

```bash
git add src/model_d.rs src/model_d/contour.rs src/model_d/mixer.rs
git commit -m "feat: add Model D contours and mixer"
```

### Task 3: Nonlinear oversampled ladder

**Files:**
- Create: `src/model_d/ladder.rs`
- Modify: `src/model_d.rs`

- [x] **Step 1: Add ladder contract tests**

Tests require:

```rust
pub enum LadderMode { Linear, Nonlinear }
pub struct LadderConfig {
    pub resonance: f32,
    pub drive: f32,
    pub mode: LadderMode,
}
pub struct ModelDLadder { /* four states, cutoff table, decimator state */ }
impl ModelDLadder {
    pub fn new(sample_rate: f32, config: LadderConfig) -> Result<Self, ModelDError>;
    pub fn sample(&mut self, input: f32, cutoff_normalized: f32) -> f32;
    pub fn reset(&mut self);
}
```

Exercise deterministic reset, allocation-free finite sampling, impulse decay,
monotonic cutoff at calibration points, cutoff error at most 8%, 20–28 dB per
octave stop-band slope, increasing resonance peak, nonlinear harmonic
generation relative to linear mode, and stability throughout the declared
cutoff/resonance/drive range.

- [ ] **Step 2: Run focused tests and verify RED** — historical command output
  was not preserved; no retrospective RED claim is made.

Run: `cargo test model_d::ladder -- --nocapture`

Expected: compile failure because `ModelDLadder` does not exist.

- [x] **Step 3: Implement the ladder**

Build a fixed 2,049-entry cutoff/coefficient table during construction. For
each host sample, linearly interpolate the table coefficient and run four
internal Euler/TPT-style substeps through four cascaded one-pole states.
Apply the bounded odd transfer at the input differential stage and at each
nonlinear stage. Return the 63-tap Blackman-windowed FIR decimator output; its
group delay is 31 internal samples, or 7.75 host samples. Clamp controls during
preparation and silence non-finite defensive output.

- [x] **Step 4: Run ladder and character nonlinear regressions**

Run:

```bash
cargo test model_d::ladder -- --nocapture
cargo test dsp::character::tests -- --nocapture
```

Expected: all selected tests pass.

- [x] **Step 5: Commit**

```bash
git add src/model_d.rs src/model_d/ladder.rs
git commit -m "feat: model nonlinear transistor ladder"
```

### Task 4: Complete monophonic character voice

**Files:**
- Create: `src/model_d/voice.rs`
- Modify: `src/model_d.rs`

- [x] **Step 1: Add complete-voice contract tests**

Define `ModelDPatch`, `ModelDDiagnostics`, and `ModelDVoice`. Tests construct
the authored bass patch, call `note_on`, render sustain, call `note_off`, and
assert deterministic reset, finiteness, exact return to idle zero, peak below
the declared internal bound, and allocation-free `sample()`. Additional tests
must prove that linear mixer, linear ladder, and no-drift/no-feedback modes each
produce a nonzero residual against the full voice while preserving duration
and pitch.

- [ ] **Step 2: Run focused tests and verify RED** — historical command output
  was not preserved; no retrospective RED claim is made.

Run: `cargo test model_d::voice -- --nocapture`

Expected: compile failure because the voice types do not exist.

- [x] **Step 3: Implement patch preparation and voice data flow**

Implement three authored patch constructors (`bass`, `lead`, and
`filter_articulation`), note lifecycle, velocity, VCO summation, feedback into
the mixer, filter-contour cutoff mapping, ladder processing, loudness contour,
fixed output gain, exact idle zero, reset, and diagnostic substitutions.

- [x] **Step 4: Run all model unit tests**

Run: `cargo test model_d -- --nocapture`

Expected: all Model D tests pass.

- [x] **Step 5: Commit**

```bash
git add src/model_d.rs src/model_d/voice.rs
git commit -m "feat: assemble Model D character voice"
```

### Task 5: Rendering, measurements, and alias evidence

**Files:**
- Create: `src/model_d_lab.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add lab-library contract tests**

Tests require exactly seven `AuditionKind` values and filenames, fixed 48 kHz
stereo rendering, shared bass score/gain for the four matched ablations,
finite/peak/DC/jump/headroom/return-to-zero gates, deterministic hashes, and
nonzero ablation residuals. Add nonlinear alias/foldback evidence with bounds
no greater than -45 dB for notes 36/60 and -35 dB for note 84.

Use:

```rust
pub struct ModelDRender { pub samples: Vec<f32>, pub metrics: ModelDMetrics }
pub fn render_audition(kind: AuditionKind, sample_rate: u32)
    -> Result<ModelDRender, ModelDError>;
pub fn measure_alias_residual(note: u8) -> Result<f64, ModelDError>;
```

- [ ] **Step 2: Run the focused test and verify RED** — historical command
  output was not preserved; no retrospective RED claim is made.

Run: `cargo test model_d_lab -- --nocapture`

Expected: compile failure because `model_d_lab` does not exist.

- [x] **Step 3: Implement scores, renderer, metrics, and alias comparison**

Implement three authored monophonic scores with explicit note-on/note-off
frames and a long enough zero tail. Render files 4–7 from the bass score and
patch with only the named diagnostic mechanism disabled. Measure peak, RMS,
DC, maximum jump, finite state, final zero, hash, pitch, and pairwise residual.
For alias evidence, use native-rate Blackman-Harris spectra over `N = 131072`
steady-state samples. Measure the 48 kHz nonharmonic out-of-mask foldback proxy
and a native 192 kHz floor in the same physical 0–24 kHz band. Mask expected
harmonics by four bins on either side, classify estimates within 6 dB of the
reference as floor-limited, and report mask coverage plus the blind spot that
energy inside masked bins is not bounded. Retain the aligned time-domain
48/192 kHz residual only as a transfer-conflated diagnostic.

- [x] **Step 4: Run library and allocation tests**

Run:

```bash
cargo test model_d_lab -- --nocapture
cargo test model_d -- --nocapture
```

Expected: all selected tests pass.

- [x] **Step 5: Commit**

```bash
git add src/lib.rs src/model_d_lab.rs
git commit -m "feat: add Model D render evidence"
```

### Task 6: Deterministic CLI and disposable listening set

**Files:**
- Create: `src/bin/model-d-lab.rs`
- Create: `tests/model_d_cli.rs`

- [x] **Step 1: Add the CLI integration test**

Run `model-d-lab render-test` into two temporary directories. Require the exact
seven WAV names, `README.md`, `manifest.tsv`, `metrics.tsv`, `ablations.tsv`,
`oscillators.tsv`, `filter.tsv`, `alias.tsv`, `hashes.tsv`, and
`generation-summary.tsv`; exclude only `workstation-cost.txt` from byte
comparison. Verify stereo float 48 kHz WAVs, identical lengths for the matched
bass files, fixed gains in the manifest, passing evidence rows, and README
statements that there is no normalization, limiter, copied preset, hardware
equivalence claim, or production integration.

- [ ] **Step 2: Run the CLI test and verify RED** — historical command output
  was not preserved; no retrospective RED claim is made.

Run: `cargo test --test model_d_cli -- --nocapture`

Expected: failure because `CARGO_BIN_EXE_model-d-lab` is unavailable.

- [x] **Step 3: Implement the artifact writer**

Accept only `render` or `render-test` plus an output directory. Render the
seven files, evaluate every gate before writing WAVs, write deterministic TSV
and Markdown reports with stable ordering and six-decimal floating formatting,
then write timing separately to `workstation-cost.txt`.

- [x] **Step 4: Run CLI test and generate the actual disposable batch**

Run:

```bash
cargo test --test model_d_cli -- --nocapture
cargo run --release --bin model-d-lab -- render artifacts/model-d-character
```

Expected: CLI test passes and the ignored artifact directory contains exactly
seven passing WAVs plus the declared reports.

- [x] **Step 5: Commit**

```bash
git add src/bin/model-d-lab.rs tests/model_d_cli.rs
git commit -m "feat: render Model D character audition"
```

### Task 7: Durable documentation and full verification

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/HANDOFF.md`
- Modify: `/home/shome/Documents/knowledge/Moj Sint/01 Current State.md`

- [x] **Step 1: Update canonical repository documentation**

Record the implemented signal path, source provenance, exact engineering
evidence, limitations, artifact location, and listening questions. State that
the result is circuit-informed, not hardware-calibrated, and remains isolated
from `Engine`.

- [x] **Step 2: Update and validate project knowledge**

Update the concise current-state note only after `docs/HANDOFF.md`; do not add
HEAD, ahead/behind state, or disposable file inventory. Run:

```bash
/home/shome/Documents/knowledge/.zk/validate.sh
```

Expected: validation exits 0.

- [x] **Step 3: Run fresh complete verification**

Run:

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo audit
cargo deny check
cargo check --target aarch64-unknown-linux-gnu --all-targets --all-features
git diff --check
```

Expected: every command exits 0; `cargo deny` may retain only the repository's
already accepted duplicate-`winnow` warning.

- [x] **Step 4: Verify deterministic release regeneration and artifact hygiene**

Generate into two fresh temporary directories with the release binary, compare
all files except `workstation-cost.txt`, compare them with
`artifacts/model-d-character`, and enumerate the artifact root. Expected:
byte-identical deterministic files and no generated material tracked by Git.

- [x] **Step 5: Commit**

```bash
git add docs/HANDOFF.md docs/RESEARCH.md
git commit -m "docs: record Model D character gate"
```

Commit the knowledge note separately in its owning notebook repository if that
notebook is version-controlled; otherwise leave the validated note as the
agent-managed workspace update.
