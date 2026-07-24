# Coupled Wire Controlled Thump Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the rejected Coupled Wire envelope/motion artifact with one
deterministic successor whose centered sub/fundamental stays linear while a
brief 105-500 Hz branch supplies bounded hard-crest thump.

**Architecture:** Add an isolated scalar `coupled_wire_thump` research module
around the existing unpresented `StruckObject::CoupledWire`. It performs
complementary prepared splits, a deterministic low-strike contour, transient
upper-bass crest shaping, post-shaper low-leakage control, offline evidence and
configuration selection. A dedicated CLI writes exactly one passing WAV and
reports; no production `Engine`, preset, JACK, ALSA, or SHR-DAW path changes.

**Tech Stack:** Stable scalar Rust, existing `StruckObject` and analysis
helpers, `assert_no_alloc`, `hound`, Cargo integration tests, release CLI
generation.

---

## File map

- Create `src/coupled_wire_thump.rs`: callback-safe voice, fixed configuration
  inventory, offline render traces, band/headroom/alias measurements,
  rejection model, and least-nonlinear passing selection.
- Modify `src/lib.rs`: export only the isolated research module.
- Create `src/bin/coupled-wire-thump-lab.rs`: deterministic one-WAV lab and
  complete report writer.
- Create `tests/coupled_wire_thump_cli.rs`: black-box artifact, format,
  rejection, content, and determinism contract.
- Modify `docs/HANDOFF.md`, `docs/RESEARCH.md`, and
  `docs/COMPOSITE_MACHINE_RESEARCH.md`: record implementation evidence and
  leave musical acceptance open.
- Modify this plan only to mark completed checkboxes.

### Task 1: Implement the prepared split and transient voice test-first

**Files:**
- Create: `src/coupled_wire_thump.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add the module export and failing source/path tests**

Add `pub mod coupled_wire_thump;` to `src/lib.rs`. Create
`src/coupled_wire_thump.rs` with the public contract and tests before the
implementation:

```rust
use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::struck_object::{StruckError, StruckObject, StruckTopology};

pub const LOW_SPLIT_HZ: f32 = 105.0;
pub const THUMP_SPLIT_HZ: f32 = 500.0;
pub const LOW_STRIKE_GAIN: f32 = 0.794_328_2;
pub const WET_START_MS: u32 = 8;
pub const WET_FULL_MS: u32 = 12;
pub const WET_HOLD_END_MS: u32 = 45;
pub const WET_END_MS: u32 = 70;
pub const LOW_HOLD_END_MS: u32 = 80;
pub const LOW_RECOVERY_END_MS: u32 = 150;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumpConfig {
    pub slug: &'static str,
    pub drive: f32,
    pub threshold: f32,
    pub wet: f32,
}

pub const CONFIGS: [ThumpConfig; 3] = [
    ThumpConfig { slug: "gentle", drive: 1.5, threshold: 0.16, wet: 0.25 },
    ThumpConfig { slug: "controlled", drive: 1.8, threshold: 0.15, wet: 0.30 },
    ThumpConfig { slug: "hard", drive: 2.1, threshold: 0.14, wet: 0.35 },
];

#[derive(Clone, Copy, Debug, Default)]
pub struct ThumpFrame {
    pub source: StereoFrame,
    pub output: StereoFrame,
    pub clean_low: f32,
    pub nonlinear_residual: StereoFrame,
}

pub struct ControlledThumpVoice {
    // fixed prepared state only
}

impl ControlledThumpVoice {
    pub fn new(config: ThumpConfig, sample_rate: u32) -> Result<Self, StruckError> {
        todo!()
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        self.sample_trace().output
    }

    #[inline]
    pub fn sample_trace(&mut self) -> ThumpFrame {
        todo!()
    }

    pub fn duration_frames(&self) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn source_before_presentation_is_sample_identical() {
        let sample_rate = 48_000;
        let mut expected =
            StruckObject::new(StruckTopology::CoupledWire, sample_rate).unwrap();
        let mut voice = ControlledThumpVoice::new(CONFIGS[0], sample_rate).unwrap();
        for _ in 0..voice.duration_frames() {
            let trace = voice.sample_trace();
            assert_eq!(trace.source.left.to_bits(), expected.sample().left.to_bits());
        }
    }

    #[test]
    fn sample_path_is_finite_allocation_free_and_returns_to_zero() {
        let mut voice = ControlledThumpVoice::new(CONFIGS[1], 48_000).unwrap();
        assert_no_alloc(|| {
            for _ in 0..voice.duration_frames() {
                let frame = voice.sample();
                assert!(frame.left.is_finite() && frame.right.is_finite());
            }
            assert_eq!(voice.sample(), StereoFrame::default());
        });
    }

    #[test]
    fn nonlinear_residual_is_confined_to_the_declared_window() {
        let sample_rate = 48_000;
        let mut voice = ControlledThumpVoice::new(CONFIGS[2], sample_rate).unwrap();
        for frame in 0..voice.duration_frames() {
            let trace = voice.sample_trace();
            let residual = trace.nonlinear_residual.left.abs()
                + trace.nonlinear_residual.right.abs();
            if frame < WET_START_MS as usize * sample_rate as usize / 1_000
                || frame >= WET_END_MS as usize * sample_rate as usize / 1_000
            {
                assert_eq!(residual.to_bits(), 0.0_f32.to_bits());
            }
        }
    }
}
```

In the equality test, cache the expected frame once per loop before comparing
both channels; do not call `expected.sample()` twice.

- [x] **Step 2: Run the focused tests and confirm RED**

Run:

```bash
cargo test coupled_wire_thump::tests --lib
```

Expected: compile failure or test failure because the voice state and sampling
implementation are absent.

- [x] **Step 3: Implement the minimal prepared voice**

Implement:

```rust
#[derive(Clone, Copy)]
struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(hz: f32, sample_rate: u32) -> Self {
        Self {
            coefficient: (-std::f32::consts::TAU * hz / sample_rate as f32).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.state =
            (1.0 - self.coefficient) * input + self.coefficient * self.state;
        self.state
    }
}

#[derive(Clone, Copy)]
struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    fn new(sample_rate: u32) -> Self {
        Self {
            coefficient:
                (-std::f32::consts::TAU * 35.0 / sample_rate as f32).exp(),
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        let output =
            input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

#[inline]
fn hard_crest(input: f32, config: ThumpConfig) -> f32 {
    (input * config.drive).clamp(-config.threshold, config.threshold)
}

#[inline]
fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}
```

Store one 105 Hz low-pass for the mono low body; one 500 Hz low-pass per
channel for the upper residual; one 105 Hz low-leakage remover per shaped
channel; two post-shaper DC blockers; `frame`, `duration_frames`,
`sample_rate`, `config`, and the exact `StruckObject`.

For each sample:

```rust
let source = self.body.sample();
let mid = 0.5 * (source.left + source.right);
let clean_low = self.low_split.sample(mid);
let upper_left = source.left - clean_low;
let upper_right = source.right - clean_low;
let thump_left = self.thump_split[0].sample(upper_left);
let thump_right = self.thump_split[1].sample(upper_right);
let bright_left = upper_left - thump_left;
let bright_right = upper_right - thump_right;

let wet = self.wet_envelope();
let shaped_left =
    thump_left + wet * self.config.wet * (hard_crest(thump_left, self.config) - thump_left);
let shaped_right =
    thump_right + wet * self.config.wet * (hard_crest(thump_right, self.config) - thump_right);
let raw_residual_left = shaped_left - thump_left;
let raw_residual_right = shaped_right - thump_right;
let residual_left =
    raw_residual_left - self.residual_low[0].sample(raw_residual_left);
let residual_right =
    raw_residual_right - self.residual_low[1].sample(raw_residual_right);
let low = clean_low * self.low_contour();

let output = StereoFrame {
    left: self.post_dc[0].sample(low + bright_left + thump_left + residual_left),
    right: self.post_dc[1].sample(low + bright_right + thump_right + residual_right),
};
```

Return exact zeros after `duration_frames`. Prepare all coefficients in
`new()`. Validate sample rate and every config field. Use smoothstep ramps from
8-12 ms and 45-70 ms for wet contribution. Keep the low branch linear while
using the accepted 8/12/25/50 ms contour, and keep the upper branch clean while
using the accepted 0.02 strike gain through 80 ms and recovery by 130 ms.

- [x] **Step 4: Run focused and struck-object tests**

Run:

```bash
cargo test coupled_wire_thump::tests --lib
cargo test struck_object::tests --lib
```

Expected: PASS with source identity, finite output, no allocation, exact
windowing, and no struck-object regression.

- [x] **Step 5: Commit the prepared voice**

```bash
git add src/lib.rs src/coupled_wire_thump.rs
git commit -m "feat: isolate controlled Coupled Wire thump"
```

### Task 2: Add offline evidence and least-nonlinear selection test-first

**Files:**
- Modify: `src/coupled_wire_thump.rs`

- [x] **Step 1: Add failing evidence and rejection tests**

Add these public types:

```rust
#[derive(Clone, Debug)]
pub struct ThumpPreview {
    pub config: ThumpConfig,
    pub samples: Vec<f32>,
    pub clean_low: Vec<f32>,
    pub nonlinear_residual: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Debug)]
pub struct ThumpRender {
    pub config: ThumpConfig,
    pub gain: f32,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct ThumpEvidence {
    pub total_rms_dbfs: f64,
    pub sample_peak_dbfs: f64,
    pub true_peak_dbfs: f64,
    pub low_onset_reduction_db: f64,
    pub low_nonlinear_residual_db: f64,
    pub thump_residual_db: f64,
    pub high_rate_residual_db: f64,
    pub reference_high_rate_residual_db: f64,
    pub metrics: crate::hybrid_subset::SubsetMetrics,
    pub tonal_pass_fraction: f64,
    pub spectral_flatness: f64,
    pub returns_to_zero: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThumpRejection {
    NonFinite,
    WeakOrExcessiveRms,
    SamplePeak,
    TruePeak,
    CeilingContact,
    LowOnset,
    LowNonlinearLeakage,
    ThumpResidual,
    DcOrJump,
    TonalInventory,
    NoiseLike,
    StereoMono,
    Aliasing,
    ReturnToZero,
}
```

Add tests:

```rust
#[test]
fn least_nonlinear_passing_configuration_meets_every_contract() {
    let selected = select_candidate(48_000).unwrap();
    let evidence = evaluate(&selected);
    assert!(evidence.rejection_reasons().is_empty(), "{evidence:?}");
    assert!((-15.5..=-11.5).contains(&evidence.total_rms_dbfs));
    assert!(evidence.sample_peak_dbfs <= -2.0);
    assert!(evidence.true_peak_dbfs <= -1.5);
    assert!((-4.0..=-1.5).contains(&evidence.low_onset_reduction_db));
    assert!(evidence.low_nonlinear_residual_db <= -40.0);
    assert!((-30.0..=-12.0).contains(&evidence.thump_residual_db));
    assert!(evidence.high_rate_residual_db <= -50.0);
}

#[test]
fn deliberately_weak_and_noisy_evidence_is_rejected() {
    let selected = select_candidate(48_000).unwrap();
    let mut evidence = evaluate(&selected);
    evidence.total_rms_dbfs = -30.0;
    evidence.spectral_flatness = 0.9;
    evidence.tonal_pass_fraction = 0.0;
    let reasons = evidence.rejection_reasons();
    assert!(reasons.contains(&ThumpRejection::WeakOrExcessiveRms));
    assert!(reasons.contains(&ThumpRejection::NoiseLike));
}

#[test]
fn selection_is_exactly_deterministic() {
    let first = select_candidate(48_000).unwrap();
    let second = select_candidate(48_000).unwrap();
    assert_eq!(first.config, second.config);
    assert_eq!(first.gain.to_bits(), second.gain.to_bits());
    assert_eq!(
        first.samples.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        second.samples.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}
```

- [x] **Step 2: Run the focused tests and confirm RED**

Run:

```bash
cargo test coupled_wire_thump::tests --lib
```

Expected: compile failure because preview, evidence, measurements, rejections,
and selection are not implemented.

- [x] **Step 3: Implement offline render, measurement, and selection**

Implement:

```rust
pub const PRESENTATION_GAINS: [f32; 9] =
    [5.00, 4.90, 4.80, 4.70, 4.60, 4.50, 4.40, 4.30, 4.20];
const SAMPLE_PEAK: f32 = 0.794_328_2;
const TRUE_PEAK: f64 = 0.841_395_1;

pub fn preview(config: ThumpConfig, sample_rate: u32)
    -> Result<ThumpPreview, StruckError>;
pub fn render(preview: &ThumpPreview, gain: f32)
    -> Result<ThumpRender, StruckError>;
pub fn evaluate(render: &ThumpRender) -> ThumpEvidence;
pub fn select_candidate(sample_rate: u32) -> Result<ThumpRender, StruckError>;
pub fn measure_high_rate_residual(config: ThumpConfig, sample_rate: u32)
    -> Result<f64, StruckError>;
pub fn measure_rejected_reference_high_rate_residual(sample_rate: u32)
    -> Result<f64, StruckError>;
```

`preview()` records interleaved output, duplicated centered low stem, and
post-low-removal nonlinear residual. `render()` applies only linear gain and
rejects any sample exceeding `SAMPLE_PEAK`; it does not clamp.

Measure:

- whole-file RMS and sample peak in dBFS;
- an eight-phase, 32-tap independently defined Hann-windowed sinc
  inter-sample peak estimate, including original sample positions;
- 45-90 Hz onset RMS through deterministic prepared filters for both the
  rejected 4.43/clamped reference and candidate;
- below-105 Hz nonlinear residual relative to clean low stem;
- 105-500 Hz nonlinear residual relative to the clean thump branch;
- existing `SubsetMetrics`, tonal pass fraction, and spectral flatness;
- exact final zero; and
- fitted 48 kHz versus 384 kHz residual over the complete 0-100 ms onset,
  using windowed-sinc 8x decimation and fitted scalar/DC removal for both the
  new voice and the rejected full-band clamp, normalized to probe-input
  energy.

Use existing declared bounds for DC, maximum jump, correlation, mono loss,
tonal fraction, and conjunctive noise classification. `rejection_reasons()`
must implement every design threshold. `select_candidate()` iterates
`CONFIGS` from gentle to hard and `PRESENTATION_GAINS` from hot to lower,
returning the first configuration/gain pair with no reasons.

- [x] **Step 4: Run focused tests and Clippy**

Run:

```bash
cargo test coupled_wire_thump::tests --lib
cargo clippy --lib --all-features -- -D warnings
```

Expected: PASS. If no candidate passes, inspect the individual reason vector
and change one causal configuration value only; do not relax rejection bounds
or stack fixes.

- [x] **Step 5: Commit evidence and selection**

```bash
git add src/coupled_wire_thump.rs
git commit -m "test: reject uncontrolled Coupled Wire thump"
```

### Task 3: Add the deterministic one-WAV lab test-first

**Files:**
- Create: `src/bin/coupled-wire-thump-lab.rs`
- Create: `tests/coupled_wire_thump_cli.rs`

- [x] **Step 1: Write the failing CLI integration test**

Create `tests/coupled_wire_thump_cli.rs`:

```rust
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn lab_writes_one_deterministic_passing_controlled_thump() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());

    let files = deterministic_files(first.path());
    assert_eq!(files, deterministic_files(second.path()));
    assert_eq!(
        files.keys().filter(|name| name.ends_with(".wav")).collect::<Vec<_>>(),
        ["01_coupled_wire_controlled_thump.wav"]
    );
    for report in [
        "README.md",
        "manifest.tsv",
        "metrics.tsv",
        "bands.tsv",
        "alias.tsv",
        "selection.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let mut wav = hound::WavReader::open(
        first.path().join("01_coupled_wire_controlled_thump.wav")
    ).unwrap();
    assert_eq!(wav.spec().channels, 2);
    assert_eq!(wav.spec().sample_rate, 48_000);
    assert_eq!(wav.spec().sample_format, hound::SampleFormat::Float);
    assert_eq!(wav.samples::<f32>().count(), 2 * 48_000 * 1_600 / 1_000);

    let rejection = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    assert!(rejection.contains("\\tpass\\tnone"));
    let readme = fs::read_to_string(first.path().join("README.md")).unwrap();
    assert!(readme.contains("one listening file"));
    assert!(readme.contains("sub and fundamental remain linear"));
    assert!(readme.contains("external playback chain"));
}

fn run_lab(output: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_coupled-wire-thump-lab"))
        .arg("render-test")
        .arg(output)
        .status()
        .unwrap();
    assert!(status.success());
}

fn deterministic_files(directory: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.file_name() != "workstation-cost.txt")
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (name, fs::read(entry.path()).unwrap())
        })
        .collect()
}
```

- [x] **Step 2: Run the CLI test and confirm RED**

Run:

```bash
cargo test --test coupled_wire_thump_cli
```

Expected: compile failure because `CARGO_BIN_EXE_coupled-wire-thump-lab` and the
binary do not exist.

- [x] **Step 3: Implement the lab and reports**

Create `src/bin/coupled-wire-thump-lab.rs` following the existing Coupled Wire
lab pattern. Accept only:

```text
coupled-wire-thump-lab <render|render-test> <output-directory>
```

Call `select_candidate(48_000)`, evaluate it, and write:

- `01_coupled_wire_controlled_thump.wav` only when rejection reasons are empty;
- `manifest.tsv`: file, source, duration, config, gain, status;
- `metrics.tsv`: RMS, sample/inter-sample peaks, DC, jump, stereo, mono,
  tonal, flatness, finite, final zero;
- `bands.tsv`: low-onset reduction, low nonlinear leakage, thump residual,
  split frequencies and transient windows;
- `alias.tsv`: candidate residual relative to probe input, the absolute -50 dB
  acceptance bound, and the rejected-reference residual for comparison;
- `selection.tsv`: every config/gain attempt and its rejection reasons;
- `rejections.tsv`: final pass/reject;
- `hashes.tsv`: FNV-1a sample hash;
- `generation-summary.tsv`: one candidate, one passing WAV, no full-band
  limiter, no production integration;
- `README.md`: one listening file, clean centered low branch, transient
  upper-bass hard crest, digital-level versus acoustic-chain limitation; and
- `workstation-cost.txt`: scalar x86_64 timing labeled as workstation-only.

Write stereo 32-bit float WAV using `hound`. If the selected render fails when
re-evaluated, write reports but no WAV and return failure.

- [x] **Step 4: Run deterministic CLI and focused unit tests**

Run:

```bash
cargo test --test coupled_wire_thump_cli
cargo test coupled_wire_thump::tests --lib
cargo clippy --bin coupled-wire-thump-lab --test coupled_wire_thump_cli -- -D warnings
```

Expected: PASS with exactly one deterministic WAV and all reports.

- [x] **Step 5: Commit the lab**

```bash
git add src/bin/coupled-wire-thump-lab.rs tests/coupled_wire_thump_cli.rs
git commit -m "feat: present one controlled Coupled Wire thump"
```

### Task 4: Generate, inspect, replace artifacts, and document evidence

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Modify: `docs/superpowers/plans/2026-07-24-coupled-wire-controlled-thump.md`
- Generate ignored: `artifacts/coupled-wire-controlled-thump/*`
- Trash rejected ignored: `artifacts/coupled-wire-envelope-motion/`

- [x] **Step 1: Build release and generate two temporary batches**

Run:

```bash
cargo build --release --bin coupled-wire-thump-lab
first=$(mktemp -d /tmp/moj-controlled-thump-a.XXXXXX)
second=$(mktemp -d /tmp/moj-controlled-thump-b.XXXXXX)
target/release/coupled-wire-thump-lab render "$first"
target/release/coupled-wire-thump-lab render "$second"
```

Expected: both commands exit 0 and each directory contains exactly one WAV.

- [x] **Step 2: Compare and inspect the complete evidence**

Compare every file except `workstation-cost.txt` byte-for-byte. Inspect
`selection.tsv`, `rejections.tsv`, `metrics.tsv`, `bands.tsv`, and `alias.tsv`.
Confirm:

- one passing candidate and no rejected WAV;
- no sample ceiling contact;
- RMS, sample peak, inter-sample peak, low reduction, leakage, thump residual,
  absolute alias, tonal/noise, mono, decay, and final-zero bounds pass;
- the clean source hash remains exact before processing; and
- reports make no acoustic SPL or Raspberry Pi claim.

- [x] **Step 3: Replace the rejected artifact safely**

Copy the passing temporary batch to
`artifacts/coupled-wire-controlled-thump/`, compare it byte-for-byte to the
source temporary batch except workstation timing, then move
`artifacts/coupled-wire-envelope-motion/` to Trash with `gio trash`.

Confirm `find artifacts -mindepth 1 -maxdepth 1 -type d` prints exactly:

```text
artifacts/coupled-wire-controlled-thump
```

- [x] **Step 4: Record the completed engineering checkpoint**

Update all three durable docs with:

- selected config and fixed gain;
- exact band splits and timing;
- old versus new low-onset RMS;
- low nonlinear leakage and thump residual;
- sample and inter-sample peak headroom;
- whole-file RMS, DC, jump, tonal/noise, stereo/mono, and decay evidence;
- high-rate residual comparison;
- deterministic aggregate hash;
- the rejected old batch removal; and
- human listening still open with no production integration.

- [x] **Step 5: Verify docs and commit**

Run:

```bash
cargo fmt --check
git diff --check
```

Expected: exit 0.

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md \
  docs/superpowers/plans/2026-07-24-coupled-wire-controlled-thump.md
git commit -m "docs: record controlled Coupled Wire thump gate"
```

### Task 5: Run the full gate and finish the local branch

**Files:**
- Verify all tracked changes and ignored artifact contents.

- [x] **Step 1: Invoke `superpowers:verification-before-completion`**

Run the required fresh commands:

```bash
cargo fmt --check
git diff --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo audit
cargo deny check
cargo check --target aarch64-unknown-linux-gnu --all-targets --all-features
```

Expected: all commands pass. `cargo deny check` may retain only the already
documented duplicate-version warning if dependency state is unchanged.

- [x] **Step 2: Re-run deterministic release generation**

Generate two fresh release batches and compare them to each other and the
installed artifact, excluding only `workstation-cost.txt`. Re-read the
rejection report and verify exactly one WAV.

- [x] **Step 3: Inspect repository and artifact hygiene**

Run:

```bash
git status --short
git log --oneline --decorate -8
find artifacts -maxdepth 2 -type f -printf '%p\n' | sort
git check-ignore -v artifacts/coupled-wire-controlled-thump/README.md
```

Expected: clean tracked worktree, coherent commits, only the new ignored
artifact directory, and ignore evidence from `.gitignore`.

- [ ] **Step 4: Invoke `superpowers:finishing-a-development-branch`**

Use the already authorized local integration path: fast-forward local `main`,
run `cargo test --all-targets --all-features` on merged `main`, remove the
temporary worktree, delete the merged feature branch, and do not push.

- [ ] **Step 5: Report the listening gate**

Link `artifacts/coupled-wire-controlled-thump/README.md`, name the single WAV,
summarize the measured low-band control and headroom, report verification and
local commit, and ask only for the human listening verdict.
