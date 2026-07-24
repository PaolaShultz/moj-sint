# Monophonic Composite Envelope Audition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the steady chord/progression listening batch with three short, musically monophonic renders of the exact three-single-layer composite under clearly different shared attack times, omitting any render below the whole-file RMS floor.

**Architecture:** Add an isolated `envelope_audition` renderer that prepares the exact `CrossSingle`, `SpectralSingle`, and `DualSingle` layers with retained 0/2/5 ms offsets, sums them with equal-power trim, and applies one profile-driven post-sum envelope. Keep the historical `hybrid_subset` renderer intact for reproducibility, but replace the lab CLI presentation with the three-profile audition and a shared fixed gain selected against whole-file RMS and sparse-ceiling constraints.

**Tech Stack:** Rust 2024, existing Moj Sint original-layer renderer, `assert_no_alloc`, `hound`, Cargo integration tests.

---

### Task 1: Lock the monophonic source and envelope contracts

**Files:**
- Create: `src/envelope_audition.rs`
- Modify: `src/lib.rs`
- Test: `src/envelope_audition.rs`

- [ ] **Step 1: Add failing profile/source tests**

Add module tests requiring:

```rust
#[test]
fn audition_uses_only_exact_single_note_layers() {
    assert_eq!(
        MONOPHONIC_LAYERS,
        [
            OriginalLayer::CrossSingle,
            OriginalLayer::SpectralSingle,
            OriginalLayer::DualSingle,
        ]
    );
    assert_eq!(LAYER_OFFSETS_MS, [0, 2, 5]);
    assert!(
        MONOPHONIC_LAYERS
            .iter()
            .all(|layer| layer.condition() == HybridCondition::Single)
    );
}

#[test]
fn attack_profiles_are_short_fixed_and_share_the_remaining_shape() {
    assert_eq!(AttackProfile::ALL.map(AttackProfile::attack_ms), [6, 35, 140]);
    for profile in AttackProfile::ALL {
        assert_eq!(profile.duration_ms(), 2_400);
        assert_eq!(profile.decay_ms(), 220);
        assert_eq!(profile.sustain(), 0.58);
        assert_eq!(profile.release_ms(), 500);
        assert!(LAYER_OFFSETS_MS.iter().all(|offset| *offset < profile.attack_ms()));
    }
}
```

Expose `pub mod envelope_audition;` from `src/lib.rs`.

Run:

```bash
cargo test --lib envelope_audition::tests::audition_uses_only_exact_single_note_layers
```

Expected: compilation fails because the module and profile types do not exist.

- [ ] **Step 2: Implement the minimal profile vocabulary**

Create:

```rust
pub const MONOPHONIC_LAYERS: [OriginalLayer; 3] = [
    OriginalLayer::CrossSingle,
    OriginalLayer::SpectralSingle,
    OriginalLayer::DualSingle,
];
pub const LAYER_OFFSETS_MS: [u32; 3] = [0, 2, 5];
pub const DURATION_MS: u32 = 2_400;
pub const DECAY_MS: u32 = 220;
pub const SUSTAIN: f32 = 0.58;
pub const RELEASE_MS: u32 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackProfile {
    VeryShort,
    Moderate,
    Slow,
}

impl AttackProfile {
    pub const ALL: [Self; 3] = [Self::VeryShort, Self::Moderate, Self::Slow];

    pub const fn attack_ms(self) -> u32 {
        match self {
            Self::VeryShort => 6,
            Self::Moderate => 35,
            Self::Slow => 140,
        }
    }
}
```

Add constant accessors for duration, decay, sustain, release, slug, label, and
the exact filenames declared in the design.

- [ ] **Step 3: Run the focused source/profile tests**

```bash
cargo test --lib envelope_audition::tests::audition_
cargo fmt --check
```

Expected: both profile/source tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/envelope_audition.rs src/lib.rs
git commit -m "feat: define monophonic envelope audition profiles"
```

### Task 2: Implement the shared post-sum envelope test-first

**Files:**
- Modify: `src/envelope_audition.rs`
- Test: `src/envelope_audition.rs`

- [ ] **Step 1: Add a failing sample-path contract**

Add:

```rust
#[test]
fn complete_sum_uses_one_finite_allocation_free_envelope() {
    for profile in AttackProfile::ALL {
        let mut mixer = EnvelopeAuditionMixer::new(profile, 8_000, 1.0).unwrap();
        assert_eq!(mixer.duration_frames(), 19_200);
        let first = assert_no_alloc(|| mixer.sample());
        assert_eq!(first, HybridFrame::default());
        for _ in 0..mixer.duration_frames().saturating_sub(2) {
            let frame = assert_no_alloc(|| mixer.sample());
            assert!(frame.left.is_finite() && frame.right.is_finite());
        }
        let final_frame = assert_no_alloc(|| mixer.sample());
        assert_eq!(final_frame, HybridFrame::default());
        assert_eq!(mixer.last_envelope(), 0.0);
    }
}
```

Run:

```bash
cargo test --lib envelope_audition::tests::complete_sum_uses_one_finite_allocation_free_envelope
```

Expected: compilation fails because `EnvelopeAuditionMixer` is absent.

- [ ] **Step 2: Implement prepared layer and envelope state**

Add `PreparedLayer`, `PreparedEnvelope`, and `EnvelopeAuditionMixer`.
Construction renders only `MONOPHONIC_LAYERS`, converts fixed offsets to
frames, precomputes attack/decay/release frame counts, fixes the render length
to `DURATION_MS`, and precomputes `1 / sqrt(3)` layer trim.

The allocation-free `sample_raw` operation must:

```rust
let mut sum = HybridFrame::default();
// Add each active exact layer using original_composite_layer_gain().
sum.left *= self.layer_trim;
sum.right *= self.layer_trim;
let envelope = self.envelope.value_at(self.frame, self.duration_frames);
self.last_envelope = envelope;
self.frame = self.frame.saturating_add(1);
HybridFrame {
    left: sum.left * envelope,
    right: sum.right * envelope,
}
```

`sample` then applies the caller-supplied presentation gain and the existing
`OUTPUT_CEILING` to both channels.

- [ ] **Step 3: Run focused envelope and allocation tests**

```bash
cargo test --lib envelope_audition::tests
cargo clippy --lib -- -D warnings
```

Expected: all audition tests pass with no warning.

- [ ] **Step 4: Commit**

```bash
git add src/envelope_audition.rs
git commit -m "feat: render one enveloped monophonic composite"
```

### Task 3: Enforce whole-file RMS rejection and fair shared gain

**Files:**
- Modify: `src/envelope_audition.rs`
- Test: `src/envelope_audition.rs`

- [ ] **Step 1: Add failing total-RMS tests**

Add:

```rust
#[test]
fn total_rms_counts_the_complete_render() {
    let samples = [1.0_f32, 1.0, 0.0, 0.0];
    assert!((measure_total_rms(&samples) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1.0e-12);
}

#[test]
fn low_total_rms_is_rejected_even_when_active_rms_is_high() {
    assert!(total_rms_is_too_low(0.19));
    assert!(!total_rms_is_too_low(MIN_TOTAL_RMS));
}
```

Run:

```bash
cargo test --lib envelope_audition::tests::total_rms_counts_the_complete_render
cargo test --lib envelope_audition::tests::low_total_rms_is_rejected_even_when_active_rms_is_high
```

Expected: compilation fails because total-RMS evidence is absent.

- [ ] **Step 2: Implement preview, shared-gain, render, and evidence types**

Add:

```rust
pub const MIN_TOTAL_RMS: f64 = 0.199_526_23; // -14 dBFS
pub const MAX_TOTAL_RMS: f64 = 0.316_227_77; // -10 dBFS

pub struct EnvelopePreview {
    pub profile: AttackProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub struct EnvelopeRender {
    pub profile: AttackProfile,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub gain: f32,
    pub total_rms: f64,
    pub metrics: SubsetMetrics,
}
```

Implement `preview_all`, `select_shared_gain`, `render_profile`, and
`evaluate_render`. Implement
`total_rms_is_too_low(total_rms) -> bool` as the single lower-bound predicate
used by evidence and its regression test. Gain selection uses one quarter-step fixed gain for all
three previews and only accepts gains for which every profile is at or below
`MAX_TOTAL_RMS` and `MAX_CEILING_PROPORTION`.

`EnvelopeEvidence::rejection_reasons` rejects total RMS below
`MIN_TOTAL_RMS`, non-finite samples, excessive DC/jump/ceiling, failed
stereo/mono, tonal coverage below two thirds, conjunctive noise classification,
or a non-zero final stereo frame.

- [ ] **Step 3: Prove every release profile passes the total-RMS floor**

Add:

```rust
#[test]
fn shared_gain_keeps_every_profile_inside_the_total_rms_gate() {
    let previews = preview_all(48_000).unwrap();
    let gain = select_shared_gain(&previews).unwrap();
    for preview in &previews {
        let render = render_profile(preview, gain).unwrap();
        assert!((MIN_TOTAL_RMS..=MAX_TOTAL_RMS).contains(&render.total_rms));
        assert!(evaluate_render(&render).rejection_reasons().is_empty());
    }
}
```

Run:

```bash
cargo test --lib envelope_audition::tests
```

Expected: every profile passes under the one shared gain.

- [ ] **Step 4: Commit**

```bash
git add src/envelope_audition.rs
git commit -m "feat: reject weak envelope auditions by total rms"
```

### Task 4: Replace the listening CLI and artifact contract

**Files:**
- Replace: `src/bin/hybrid-subset-lab.rs`
- Move: `tests/hybrid_subset_cli.rs` to `tests/envelope_audition_cli.rs`

- [ ] **Step 1: Replace the integration test first**

Require two independent `render-test` runs to match byte-for-byte except
`workstation-cost.txt`, and require exactly:

```rust
let expected_wavs = [
    "01_monophonic_attack_006ms.wav",
    "02_monophonic_attack_035ms.wav",
    "03_monophonic_attack_140ms.wav",
];
assert_eq!(wav_names, expected_wavs);
assert!(!readme.contains("chord"));
assert!(!readme.contains("progression"));
assert!(readme.contains("musically monophonic"));
assert!(readme.contains("whole-file RMS"));
```

Parse `manifest.tsv` and assert every present row has `total_rms_dbfs >= -14`.
Parse `rejections.tsv` and assert every rejected filename is absent.

Run:

```bash
cargo test --test envelope_audition_cli
```

Expected: fail because the current CLI writes eight steady
chord/progression/reference files.

- [ ] **Step 2: Replace the CLI presentation**

Keep the existing command shape:

```text
hybrid-subset-lab <render|render-test> <output-directory>
```

Render only the three `envelope_audition` profiles. Write a WAV only when
`EnvelopeEvidence::rejection_reasons()` is empty. Generate:

- `README.md`;
- `manifest.tsv`;
- `metrics.tsv`;
- `rejections.tsv`;
- `hashes.tsv`;
- `generation-summary.tsv`;
- `workstation-cost.txt`.

Reports must include layer names, offsets, all ADSR values, shared gain, total
and active RMS, peak, ceiling proportion, tonal/noise status, stereo/mono
metrics, tail result, and deterministic hashes.

- [ ] **Step 3: Run CLI determinism and lint checks**

```bash
cargo test --test envelope_audition_cli
cargo clippy --bin hybrid-subset-lab --test envelope_audition_cli -- -D warnings
```

Expected: exactly three passing WAVs, no forbidden material, and deterministic
reports.

- [ ] **Step 4: Commit**

```bash
git add src/bin/hybrid-subset-lab.rs tests/hybrid_subset_cli.rs tests/envelope_audition_cli.rs
git commit -m "feat: present monophonic envelope comparisons"
```

### Task 5: Generate the sole final batch and update durable status

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Generate ignored: `artifacts/monophonic-envelope-composites/`

- [ ] **Step 1: Render two independent 48 kHz batches**

```bash
cargo run --release --bin hybrid-subset-lab -- render <first-temp-directory>
cargo run --release --bin hybrid-subset-lab -- render <second-temp-directory>
```

Expected: deterministic files are byte-identical except
`workstation-cost.txt`; every retained row passes the whole-file RMS floor and
all other rejection gates.

- [ ] **Step 2: Install only the verified batch**

Confirm `artifacts/` is empty, then copy one passing batch to:

```text
artifacts/monophonic-envelope-composites/
```

Assert that this is the only child of `artifacts/`, that it contains exactly
three WAVs, and that no rejected filename exists.

- [ ] **Step 3: Update durable documents**

Record the steady-envelope batch as rejected because its 0.88 sustain occupied
nearly the full long render. Record the exact monophonic source membership,
three envelope profiles, total-RMS gate, release metrics, deterministic hash,
and open human listening verdict. Do not claim an envelope is musically
selected.

- [ ] **Step 4: Commit**

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md
git commit -m "docs: record monophonic envelope audition gate"
```

### Task 6: Full verification and local integration

**Files:**
- Verify all tracked changes
- Inspect ignored `artifacts/monophonic-envelope-composites/`

- [ ] **Step 1: Run required verification**

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo audit
cargo deny check
cargo check --target aarch64-unknown-linux-gnu
git diff --check
```

Expected: every command exits 0; `cargo deny` may retain only the accepted
duplicate `winnow` warning.

- [ ] **Step 2: Verify artifact and Git hygiene**

Confirm the installed directory is ignored, contains exactly three passing
WAVs, is the only child under `artifacts/`, and that two release generations
match except workstation timing.

- [ ] **Step 3: Integrate locally without pushing**

Fast-forward verified work into local `main`, copy the verified ignored batch
to the main workspace, rerun the complete all-target/all-feature test suite,
then remove the temporary worktree and feature branch. Do not push.
