# Original Hybrid Subset Mix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate a disposable listening batch containing only three- or four-layer combinations of the exact original twelve hybrid renders, with moderately hot shared gain, sparse hard-limited crests, and automatic rejection of obvious noise or engineering defects.

**Architecture:** Expose the original hybrid-layer inventory from the existing composite reconstruction without changing its hashes. Add an isolated `hybrid_subset` research module that prepares selected original layers, preserves rebased launch offsets, applies one shared gain per layer-count group into a static ceiling, measures rejection evidence, and renders through an allocation-free sample method. A dedicated CLI writes only passing candidates plus the twelve-layer reference and concise reports under the ignored `artifacts/` tree.

**Tech Stack:** Rust 2024, existing Moj Sint hybrid/composite DSP, `hound` float WAV output, `assert_no_alloc`, Cargo integration tests.

---

### Task 1: Expose the exact original layer inventory

**Files:**
- Modify: `src/composite_machine.rs`
- Test: `src/composite_machine.rs`

- [ ] **Step 1: Write the failing inventory and reconstruction tests**

Add tests that require twelve stable original layer identities, their fixed
delayed offsets, and unchanged 48 kHz reconstruction hashes:

```rust
#[test]
fn original_layer_inventory_is_exact_and_stable() {
    assert_eq!(OriginalLayer::ALL.len(), 12);
    assert_eq!(
        OriginalLayer::ALL.map(OriginalLayer::label),
        [
            "cross-single",
            "cross-chord",
            "cross-progression",
            "cross-progression-mono",
            "spectral-single",
            "spectral-chord",
            "spectral-progression",
            "spectral-progression-mono",
            "dual-single",
            "dual-chord",
            "dual-progression",
            "dual-progression-mono",
        ]
    );
    assert_eq!(
        OriginalLayer::ALL.map(OriginalLayer::delayed_start_ms),
        delayed_launch_offsets()
    );
}

#[test]
fn public_original_layer_render_preserves_reference_hashes() {
    let synchronized =
        render_composite(CompositeCandidate::SynchronizedReference, 48_000, false).unwrap();
    let delayed =
        render_composite(CompositeCandidate::DelayedLaunchEstimate, 48_000, false).unwrap();
    assert_eq!(synchronized.metrics.sample_hash, 0xbe3e_a0fd_d66b_4472);
    assert_eq!(delayed.metrics.sample_hash, 0xc919_cb57_920c_520f);
    let layer = render_original_layer(OriginalLayer::CrossSingle, 8_000).unwrap();
    assert!(!layer.is_empty());
    assert!(layer.iter().all(|sample| sample.is_finite()));
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```bash
cargo test --lib composite_machine::tests::original_layer_inventory_is_exact_and_stable
```

Expected: compilation fails because `OriginalLayer` and
`render_original_layer` do not exist.

- [ ] **Step 3: Implement the public original-layer boundary**

Add this exact enum and mapping API near `LayerSpec`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OriginalLayer {
    CrossSingle,
    CrossChord,
    CrossProgression,
    CrossProgressionMono,
    SpectralSingle,
    SpectralChord,
    SpectralProgression,
    SpectralProgressionMono,
    DualSingle,
    DualChord,
    DualProgression,
    DualProgressionMono,
}

impl OriginalLayer {
    pub const ALL: [Self; 12] = [
        Self::CrossSingle,
        Self::CrossChord,
        Self::CrossProgression,
        Self::CrossProgressionMono,
        Self::SpectralSingle,
        Self::SpectralChord,
        Self::SpectralProgression,
        Self::SpectralProgressionMono,
        Self::DualSingle,
        Self::DualChord,
        Self::DualProgression,
        Self::DualProgressionMono,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::CrossSingle => "cross-single",
            Self::CrossChord => "cross-chord",
            Self::CrossProgression => "cross-progression",
            Self::CrossProgressionMono => "cross-progression-mono",
            Self::SpectralSingle => "spectral-single",
            Self::SpectralChord => "spectral-chord",
            Self::SpectralProgression => "spectral-progression",
            Self::SpectralProgressionMono => "spectral-progression-mono",
            Self::DualSingle => "dual-single",
            Self::DualChord => "dual-chord",
            Self::DualProgression => "dual-progression",
            Self::DualProgressionMono => "dual-progression-mono",
        }
    }

    pub const fn family(self) -> HybridFamily {
        match self {
            Self::CrossSingle
            | Self::CrossChord
            | Self::CrossProgression
            | Self::CrossProgressionMono => HybridFamily::CrossCoupledMachine,
            Self::SpectralSingle
            | Self::SpectralChord
            | Self::SpectralProgression
            | Self::SpectralProgressionMono => HybridFamily::SpectralShadow,
            Self::DualSingle
            | Self::DualChord
            | Self::DualProgression
            | Self::DualProgressionMono => HybridFamily::DualResonantBody,
        }
    }

    pub const fn condition(self) -> HybridCondition {
        match self {
            Self::CrossSingle | Self::SpectralSingle | Self::DualSingle => {
                HybridCondition::Single
            }
            Self::CrossChord | Self::SpectralChord | Self::DualChord => {
                HybridCondition::HeldChord
            }
            Self::CrossProgression
            | Self::SpectralProgression
            | Self::DualProgression => HybridCondition::Progression,
            Self::CrossProgressionMono
            | Self::SpectralProgressionMono
            | Self::DualProgressionMono => HybridCondition::ProgressionMono,
        }
    }
    pub const fn delayed_start_ms(self) -> u32 {
        DELAYED_LAUNCH_MS[self as usize]
    }
}

pub fn render_original_layer(
    layer: OriginalLayer,
    sample_rate: u32,
) -> Result<Vec<f32>, HybridRenderError> {
    prepare_layer_samples(
        LayerSpec {
            family: layer.family(),
            score: match layer.condition() {
                HybridCondition::Single => Score::Single,
                HybridCondition::HeldChord => Score::HeldChord,
                HybridCondition::Progression => Score::Progression,
                HybridCondition::ProgressionMono => Score::ProgressionMono,
            },
            start_ms: layer.delayed_start_ms(),
            label: layer.label(),
            mix_gain: ORIGINAL_COMPOSITE_GAIN,
            matrix: IDENTITY_MATRIX,
        },
        sample_rate,
    )
}

pub const fn original_composite_layer_gain() -> f32 {
    ORIGINAL_COMPOSITE_GAIN
}
```

Replace `original_layer_specs`' nested family/score loops with
`OriginalLayer::ALL.into_iter().map(|layer| original_layer_spec(layer,
delayed)).collect()`. The helper uses `0` when `delayed` is false and
`layer.delayed_start_ms()` otherwise. Keep the exact order, labels,
conditions, matrices, gain, and delayed offsets so both locked hashes remain
unchanged.

- [ ] **Step 4: Run the focused and reconstruction tests**

Run:

```bash
cargo test --lib composite_machine::tests::original_layer
cargo test --lib composite_machine::tests::hot_output_contract_uses_explicit_gains_without_changing_raw_references
```

Expected: all selected tests pass and both raw hashes remain exact.

- [ ] **Step 5: Commit the inventory boundary**

```bash
git add src/composite_machine.rs
git commit -m "refactor: expose original hybrid layer inventory"
```

### Task 2: Implement the test-first subset mixer and gain policy

**Files:**
- Create: `src/hybrid_subset.rs`
- Modify: `src/lib.rs`
- Test: `src/hybrid_subset.rs`

- [ ] **Step 1: Add failing candidate-membership and gain-policy tests**

Create `src/hybrid_subset.rs` with a test module first. Require the exact seven
candidate memberships from the design, three/four-layer limits, rebased
offsets, finite output, allocation-free sampling, a `-0.3 dBFS` ceiling, and
shared gains:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn candidates_use_only_exact_original_layers() {
        assert_eq!(SubsetCandidate::ALL.len(), 7);
        for candidate in SubsetCandidate::ALL {
            assert!((3..=4).contains(&candidate.layers().len()));
            assert!(candidate
                .layers()
                .iter()
                .all(|layer| OriginalLayer::ALL.contains(layer)));
        }
        assert_eq!(
            SubsetCandidate::ThreeSingles.layers(),
            &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(
            SubsetCandidate::CrossAnchor.layers(),
            &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualProgression,
                OriginalLayer::CrossProgressionMono,
            ]
        );
    }

    #[test]
    fn subset_sampling_is_finite_bounded_and_allocation_free() {
        let mut mixer =
            HybridSubsetMixer::new(SubsetCandidate::ThreeSingles, 8_000, 1.0).unwrap();
        for _ in 0..1_024 {
            let frame = assert_no_alloc(|| mixer.sample());
            assert!(frame.left.is_finite() && frame.right.is_finite());
            assert!(frame.left.abs() <= OUTPUT_CEILING);
            assert!(frame.right.abs() <= OUTPUT_CEILING);
        }
    }

    #[test]
    fn shared_gain_is_group_wide_and_respects_sparse_ceiling_rule() {
        let previews = preview_all(8_000).unwrap();
        let gains = select_group_gains(&previews).unwrap();
        assert!(gains.three_layer.is_finite() && gains.three_layer > 0.0);
        assert!(gains.four_layer.is_finite() && gains.four_layer > 0.0);
        for preview in previews {
            let gain = gains.for_candidate(preview.candidate);
            let metrics = preview.metrics_at_gain(gain);
            assert!(metrics.ceiling_proportion <= MAX_CEILING_PROPORTION);
        }
    }
}
```

- [ ] **Step 2: Run the module tests and verify they fail**

Run:

```bash
cargo test --lib hybrid_subset::tests
```

Expected: compilation fails because `hybrid_subset` is not exported and the
types/functions are not implemented.

- [ ] **Step 3: Implement candidates, prepared layers, and allocation-free sampling**

Export `pub mod hybrid_subset;` from `src/lib.rs`. Implement:

```rust
pub const OUTPUT_CEILING: f32 = 0.966_050_86; // -0.3 dBFS
pub const MAX_CEILING_PROPORTION: f64 = 0.01;
pub const MIN_ACTIVE_RMS: f64 = 0.199_526_23; // -14 dBFS
pub const MAX_ACTIVE_RMS: f64 = 0.316_227_77; // -10 dBFS

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubsetCandidate {
    ThreeSingles,
    ThreeChords,
    ThreeStereoProgressions,
    ThreeMonoProgressions,
    CrossAnchor,
    SpectralAnchor,
    DualAnchor,
}

impl SubsetCandidate {
    pub const ALL: [Self; 7] = [
        Self::ThreeSingles,
        Self::ThreeChords,
        Self::ThreeStereoProgressions,
        Self::ThreeMonoProgressions,
        Self::CrossAnchor,
        Self::SpectralAnchor,
        Self::DualAnchor,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::ThreeSingles => "three-singles",
            Self::ThreeChords => "three-chords",
            Self::ThreeStereoProgressions => "three-stereo-progressions",
            Self::ThreeMonoProgressions => "three-mono-progressions",
            Self::CrossAnchor => "cross-anchor",
            Self::SpectralAnchor => "spectral-anchor",
            Self::DualAnchor => "dual-anchor",
        }
    }

    pub const fn layers(self) -> &'static [OriginalLayer] {
        match self {
            Self::ThreeSingles => &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ],
            Self::ThreeChords => &[
                OriginalLayer::CrossChord,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualChord,
            ],
            Self::ThreeStereoProgressions => &[
                OriginalLayer::CrossProgression,
                OriginalLayer::SpectralProgression,
                OriginalLayer::DualProgression,
            ],
            Self::ThreeMonoProgressions => &[
                OriginalLayer::CrossProgressionMono,
                OriginalLayer::SpectralProgressionMono,
                OriginalLayer::DualProgressionMono,
            ],
            Self::CrossAnchor => &[
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralChord,
                OriginalLayer::DualProgression,
                OriginalLayer::CrossProgressionMono,
            ],
            Self::SpectralAnchor => &[
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualChord,
                OriginalLayer::CrossProgression,
                OriginalLayer::SpectralProgressionMono,
            ],
            Self::DualAnchor => &[
                OriginalLayer::DualSingle,
                OriginalLayer::CrossChord,
                OriginalLayer::SpectralProgression,
                OriginalLayer::DualProgressionMono,
            ],
        }
    }
}

#[derive(Clone)]
struct PreparedLayer {
    samples: Arc<[f32]>,
    start_frame: usize,
}

pub struct HybridSubsetMixer {
    layers: Vec<PreparedLayer>,
    frame: usize,
    duration_frames: usize,
    gain: f32,
}

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum SubsetError {
    #[error("sample rate must be positive")]
    InvalidSampleRate,
    #[error("gain must be finite and positive")]
    InvalidGain,
    #[error(transparent)]
    Hybrid(#[from] HybridRenderError),
    #[error("no positive shared gain satisfies the sparse-ceiling rule")]
    NoSharedGain,
}

impl HybridSubsetMixer {
    pub fn new(
        candidate: SubsetCandidate,
        sample_rate: u32,
        gain: f32,
    ) -> Result<Self, SubsetError> {
        if sample_rate == 0 {
            return Err(SubsetError::InvalidSampleRate);
        }
        if !gain.is_finite() || gain <= 0.0 {
            return Err(SubsetError::InvalidGain);
        }
        let first_ms = candidate
            .layers()
            .iter()
            .map(|layer| layer.delayed_start_ms())
            .min()
            .unwrap_or(0);
        let mut layers = Vec::with_capacity(candidate.layers().len());
        let mut duration_frames = 0;
        for &layer in candidate.layers() {
            let samples: Arc<[f32]> =
                render_original_layer(layer, sample_rate)?.into();
            let start_frame = ((u64::from(layer.delayed_start_ms() - first_ms)
                * u64::from(sample_rate))
                / 1_000) as usize;
            duration_frames = duration_frames.max(start_frame + samples.len() / 2);
            layers.push(PreparedLayer {
                samples,
                start_frame,
            });
        }
        Ok(Self {
            layers,
            frame: 0,
            duration_frames,
            gain,
        })
    }

    #[inline]
    pub fn sample(&mut self) -> HybridFrame {
        let mut left = 0.0;
        let mut right = 0.0;
        for layer in &self.layers {
            if let Some(local) = self.frame.checked_sub(layer.start_frame) {
                let offset = 2 * local;
                if offset + 1 < layer.samples.len() {
                    left += original_composite_layer_gain() * layer.samples[offset];
                    right += original_composite_layer_gain() * layer.samples[offset + 1];
                }
            }
        }
        self.frame += 1;
        HybridFrame {
            left: (left * self.gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING),
            right: (right * self.gain).clamp(-OUTPUT_CEILING, OUTPUT_CEILING),
        }
    }
}
```

Implement `render_raw_candidate`, `render_candidate`, active-region discovery,
sample hashing, and measurements for peak, RMS, DC, maximum jump, correlation,
mono loss, hard-ceiling proportion, and finiteness.

- [ ] **Step 4: Implement deterministic shared gain selection**

For every raw preview, sort active absolute samples and find the value at the
99th percentile. The candidate's maximum sparse-ceiling gain is
`OUTPUT_CEILING / percentile`. Select the minimum maximum gain across each
layer-count group, quantize downward to a `0.25` step, then decrement by `0.25`
until every group member measures at or below 1% hard-ceiling samples.
Reference gain uses the same algorithm on the raw delayed twelve-layer
reference alone. Return `NoSharedGain` if a positive gain cannot satisfy the
ceiling rule.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test --lib hybrid_subset::tests
```

Expected: all candidate, offset, gain, finite, bound, determinism, and
allocation tests pass.

- [ ] **Step 6: Commit the mixer**

```bash
git add src/lib.rs src/hybrid_subset.rs
git commit -m "feat: mix exact original hybrid subsets"
```

### Task 3: Add tonal, noise, stereo, and high-rate rejection evidence

**Files:**
- Modify: `src/hybrid_subset.rs`
- Test: `src/hybrid_subset.rs`

- [ ] **Step 1: Write failing rejection tests**

Add tests with synthetic buffers proving:

```rust
#[test]
fn noise_requires_flat_spectrum_and_failed_tonal_anchors() {
    let pitched_broadband = synthetic_screen(true, 0.75, true);
    assert!(!pitched_broadband.noise_like);
    let flat_unpitched = synthetic_screen(false, 0.75, false);
    assert!(flat_unpitched.noise_like);
}

#[test]
fn rejection_contract_matches_the_design_thresholds() {
    let passing = CandidateEvidence::passing_fixture();
    assert!(passing.rejection_reasons().is_empty());
    assert!(passing
        .with_ceiling_proportion(0.010_001)
        .rejection_reasons()
        .contains(&RejectionReason::ExcessiveCeiling));
    assert!(passing
        .with_active_rms(0.19)
        .rejection_reasons()
        .contains(&RejectionReason::ActiveRms));
    assert!(passing
        .with_residual_db(-0.99)
        .rejection_reasons()
        .contains(&RejectionReason::HighRateResidual));
}
```

- [ ] **Step 2: Run the rejection tests and verify they fail**

Run:

```bash
cargo test --lib hybrid_subset::tests::noise_requires_flat_spectrum_and_failed_tonal_anchors
cargo test --lib hybrid_subset::tests::rejection_contract_matches_the_design_thresholds
```

Expected: compilation fails because the evidence and rejection APIs do not
exist.

- [ ] **Step 3: Implement evidence and rejection types**

Implement `CandidateEvidence`, `RejectionReason`, and
`evaluate_candidate(candidate, render, sample_rate)`. Enforce:

```rust
const MAX_DC: f64 = 0.001;
const MAX_JUMP: f64 = 1.5;
const MAX_MONO_LOSS_DB: f64 = 1.5;
const MIN_CORRELATION: f64 = -0.25;
const MAX_HIGH_RATE_RESIDUAL_DB: f64 = -1.0;
const MAX_SPECTRAL_FLATNESS: f64 = 0.50;
const TARGET_MARGIN_DB: f64 = 36.0;
```

Measure spectral flatness over a deterministic 4,096-frame active mono window
using 256 non-DC magnitude bins. Classify noise only when flatness exceeds
`0.50` and fewer than two thirds of scheduled fundamental projections pass the
36 dB margin. Single-note segments count their one unique target once.

Measure the nonlinear hard-ceiling residual by rendering the same prepared
candidate at the target rate and eight times the target rate, advancing both
to one second after the last selected launch, averaging each eight-frame
reference group, fitting one scalar gain after DC removal, and reporting the
remaining energy ratio in dB.

- [ ] **Step 4: Run all subset tests**

Run:

```bash
cargo test --lib hybrid_subset::tests
```

Expected: all tests pass; no rejected fixture is treated as presentable.

- [ ] **Step 5: Commit rejection evidence**

```bash
git add src/hybrid_subset.rs
git commit -m "test: reject noisy hybrid subset renders"
```

### Task 4: Build the disposable subset listening CLI

**Files:**
- Create: `src/bin/hybrid-subset-lab.rs`
- Create: `tests/hybrid_subset_cli.rs`

- [ ] **Step 1: Write the failing CLI integration test**

The test runs `render-test` twice, compares every deterministic file, requires
the reference, permits only the seven designed candidate filenames, verifies
that rejected candidates have no WAV, and requires concise reports:

```rust
#[test]
fn hybrid_subset_lab_writes_only_deterministic_passing_original_combinations() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());
    assert_eq!(deterministic_files(first.path()), deterministic_files(second.path()));

    let files = deterministic_files(first.path());
    assert!(files.contains_key("01_reference_twelve-layer-delayed.wav"));
    for report in [
        "README.md",
        "manifest.tsv",
        "metrics.tsv",
        "tonal.tsv",
        "residual.tsv",
        "rejections.tsv",
        "hashes.tsv",
        "reconstruction-regression.tsv",
        "generation-summary.tsv",
    ] {
        assert!(files.contains_key(report), "missing {report}");
    }
    assert!(first.path().join("workstation-cost.txt").is_file());

    let rejections = fs::read_to_string(first.path().join("rejections.tsv")).unwrap();
    for line in rejections.lines().skip(1) {
        let fields: Vec<_> = line.split('\t').collect();
        if fields[2] == "reject" {
            assert!(!first.path().join(format!("{}_{}.wav", fields[0], fields[1])).exists());
        }
    }
}
```

- [ ] **Step 2: Run the CLI test and verify it fails**

Run:

```bash
cargo test --test hybrid_subset_cli
```

Expected: compilation fails because `CARGO_BIN_EXE_hybrid-subset-lab` is not
defined.

- [ ] **Step 3: Implement the CLI**

Support:

```text
hybrid-subset-lab render <output-directory>
hybrid-subset-lab render-test <output-directory>
```

The CLI must:

1. render raw previews;
2. select shared three-layer, four-layer, and reference gains;
3. evaluate every candidate;
4. write the reference;
5. write a candidate WAV only when `rejection_reasons()` is empty;
6. write the exact reports asserted by the test;
7. state source layers, rebased offsets, gains, active RMS, peak, ceiling
   proportion, known unchanged residual limitations, and low-volume warning in
   the README; and
8. exclude `workstation-cost.txt` from deterministic comparisons.

Use two-channel 32-bit float WAV output through `hound`. Do not write mono
diagnostic files, synchronized references, parameter variants, failed WAVs, or
new sound mechanisms.

- [ ] **Step 4: Run the CLI integration and all subset tests**

Run:

```bash
cargo test --test hybrid_subset_cli
cargo test --lib hybrid_subset::tests
```

Expected: tests pass and two quick batches are byte-identical except the
workstation timing file.

- [ ] **Step 5: Commit the CLI**

```bash
git add src/bin/hybrid-subset-lab.rs tests/hybrid_subset_cli.rs
git commit -m "feat: render original hybrid subset listening gate"
```

### Task 5: Record the rejected compact batch and generate the final disposable batch

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Delete generated files only: `artifacts/composite-machine-lab/`
- Generate untracked files: `artifacts/original-hybrid-subset-mix/`

- [ ] **Step 1: Update durable negative conclusions**

Record that the compact one-to-three-mechanism batch was rejected because none
of its sounds worked musically and its hot policy overcorrected into prolonged
5–29% clipping. Record the correction: smaller combinations must select exact
original hybrid layers, use moderate shared linear gain, and allow at most 1%
sparse static-ceiling contact.

- [ ] **Step 2: Delete the rejected disposable batch**

After confirming `artifacts/composite-machine-lab/` is ignored and contains
only generated experiment output, remove that exact directory. Do not remove
source, tests, or durable research documents.

- [ ] **Step 3: Generate two fresh release batches**

Run:

```bash
first_batch="$(mktemp -d)"
second_batch="$(mktemp -d)"
cargo run --release --bin hybrid-subset-lab -- render "$first_batch"
cargo run --release --bin hybrid-subset-lab -- render "$second_batch"
```

Expected: both commands succeed, write the same set of passing WAVs, and list
all rejected candidates only in `rejections.tsv`.

- [ ] **Step 4: Compare deterministic outputs**

Compare every file except `workstation-cost.txt`; expected result is no
differences. Review every metrics, tonal, residual, rejection, reconstruction,
and hash row. If a candidate fails, confirm its WAV is absent rather than
loosening a threshold.

- [ ] **Step 5: Replace only the final ignored listening directory**

Create `artifacts/original-hybrid-subset-mix/` from one verified fresh batch.
Do not add any artifact file to Git.

- [ ] **Step 6: Commit durable documentation**

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md
git commit -m "docs: record original hybrid subset listening gate"
```

### Task 6: Run complete verification and hand off

**Files:**
- Verify all modified tracked files
- Inspect untracked/ignored: `artifacts/original-hybrid-subset-mix/`

- [ ] **Step 1: Run formatting and all-target tests**

```bash
cargo fmt --check
cargo test --all-targets --all-features
```

Expected: exit 0 with zero failed tests.

- [ ] **Step 2: Run lint and release build**

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

Expected: exit 0 with no warnings or build failures.

- [ ] **Step 3: Run dependency and portability checks**

```bash
cargo audit
cargo deny check
cargo check --target aarch64-unknown-linux-gnu
```

Expected: audit reports no vulnerabilities; deny passes with only the already
accepted duplicate `winnow` warning if still present; AArch64 compile check
exits 0.

- [ ] **Step 4: Run hygiene and artifact checks**

```bash
git diff --check
git status --short
git check-ignore -v artifacts/original-hybrid-subset-mix/README.md
```

Expected: no tracked uncommitted changes, the listening batch is ignored, and
no WAV/report artifact is staged or committed.

- [ ] **Step 5: Inspect final evidence and commit any verification corrections**

Read the final README and every report, confirm no rejected WAV exists, record
the exact aggregate deterministic hash and passing candidate count in
`docs/HANDOFF.md`, rerun affected checks, and commit only the tracked
documentation correction.

- [ ] **Step 6: Hand off the listening gate**

Provide the exact absolute artifact path, list the reference and passing
candidate WAVs in README order, summarize rejected candidates and engineering
limits, and explicitly state that human listening—not metrics—decides whether
any subset retains the successful twelve-layer identity.
