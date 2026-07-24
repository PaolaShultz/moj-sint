# Coherent Hybrid Composite Envelope Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace second-scale sequential layer entry with fixed micro-delays and one post-sum master ADSR so every three- or four-layer candidate behaves as one complete composite sound.

**Architecture:** Keep the exact original source-layer renderer and candidate membership. Change only the isolated `hybrid_subset` timing/mix boundary: fixed 0/4/9/15 ms offsets, equal-power layer trim, one prepared master ADSR after summation, then existing shared gain and static ceiling. Replace all generated output with one ignored coherent-composite batch.

**Tech Stack:** Rust 2024, existing Moj Sint hybrid/subset modules, `assert_no_alloc`, `hound`, Cargo integration tests.

---

### Task 1: Lock micro-delay and master-envelope behavior

**Files:**
- Modify: `src/hybrid_subset.rs`
- Test: `src/hybrid_subset.rs`

- [ ] **Step 1: Add failing timing tests**

Add tests requiring:

```rust
#[test]
fn fixed_offsets_are_micro_delays_inside_the_master_attack() {
    assert_eq!(
        fixed_offsets_ms(SubsetCandidate::ThreeSingles),
        vec![0, 4, 9]
    );
    assert_eq!(
        fixed_offsets_ms(SubsetCandidate::CrossAnchor),
        vec![0, 4, 9, 15]
    );
    for candidate in SubsetCandidate::ALL {
        let offsets = fixed_offsets_ms(candidate);
        assert_eq!(offsets.len(), candidate.layers().len());
        assert!(offsets.iter().all(|offset| *offset < MASTER_ATTACK_MS));
        assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
```

Run:

```bash
cargo test --lib hybrid_subset::tests::fixed_offsets_are_micro_delays_inside_the_master_attack
```

Expected: fail because `fixed_offsets_ms` and `MASTER_ATTACK_MS` do not exist.

- [ ] **Step 2: Add failing shared-envelope tests**

Require one deterministic, finite, allocation-free master envelope whose
stereo gain starts and ends at zero:

```rust
#[test]
fn one_master_envelope_controls_the_complete_stereo_sum() {
    let mut mixer =
        HybridSubsetMixer::new(SubsetCandidate::CrossAnchor, 8_000, 1.0).unwrap();
    let first = assert_no_alloc(|| mixer.sample());
    assert_eq!(first, HybridFrame::default());
    assert_eq!(mixer.last_master_envelope(), 0.0);
    for _ in 0..mixer.duration_frames().saturating_sub(2) {
        let frame = assert_no_alloc(|| mixer.sample());
        assert!(frame.left.is_finite() && frame.right.is_finite());
    }
    let final_frame = assert_no_alloc(|| mixer.sample());
    assert_eq!(final_frame, HybridFrame::default());
    assert_eq!(mixer.last_master_envelope(), 0.0);
}
```

Run:

```bash
cargo test --lib hybrid_subset::tests::one_master_envelope_controls_the_complete_stereo_sum
```

Expected: fail because the current mixer has no shared master envelope API and
its first/last contract does not implement the approved ADSR.

- [ ] **Step 3: Implement minimal prepared master ADSR**

Add constants:

```rust
pub const MASTER_ATTACK_MS: u32 = 25;
pub const MASTER_DECAY_MS: u32 = 180;
pub const MASTER_SUSTAIN: f32 = 0.88;
pub const MASTER_RELEASE_MS: u32 = 320;
const THREE_LAYER_OFFSETS_MS: [u32; 3] = [0, 4, 9];
const FOUR_LAYER_OFFSETS_MS: [u32; 4] = [0, 4, 9, 15];
const REFERENCE_OFFSETS_MS: [u32; 12] =
    [0, 1, 3, 4, 5, 7, 8, 9, 11, 12, 14, 15];
```

Add a fixed-state `MasterEnvelope` prepared during construction. Its
`sample(frame, duration_frames)` uses linear attack, decay, sustain, and
release stages with precomputed sample counts; it performs no allocation,
locking, I/O, formatting, panic, or transcendental setup.

Change `HybridSubsetMixer::sample_raw` to:

```rust
let trim = 1.0 / (self.layers.len() as f32).sqrt();
// Sum every active micro-delayed original layer with the existing original
// composite-layer gain, then apply trim once.
output.left *= trim;
output.right *= trim;
```

Change `HybridSubsetMixer::sample` to apply the one master envelope to both
channels after `sample_raw` and before the presentation gain/static ceiling.
Store `last_master_envelope` for the focused contract test.

Replace `rebased_offsets_ms` with `fixed_offsets_ms`; construction uses
0/4/9/15 ms based only on selection order.

- [ ] **Step 4: Run focused tests**

```bash
cargo test --lib hybrid_subset::tests
```

Expected: all subset tests pass; allocation checks remain green.

- [ ] **Step 5: Commit**

```bash
git add src/hybrid_subset.rs
git commit -m "fix: make hybrid subsets one enveloped composite"
```

### Task 2: Replace the listening presentation

**Files:**
- Modify: `src/bin/hybrid-subset-lab.rs`
- Modify: `tests/hybrid_subset_cli.rs`

- [ ] **Step 1: Add failing CLI assertions**

Require:

```rust
assert!(readme.contains("fixed micro-delays"));
assert!(readme.contains("one shared master ADSR"));
assert!(!readme.contains("delayed-launch"));
assert_eq!(wav_names.len(), 8);
```

Require `manifest.tsv` to contain `0,4,9` and `0,4,9,15`, and reject any offset
greater than 15 ms.

Run:

```bash
cargo test --test hybrid_subset_cli
```

Expected: fail because the current reports describe rebased long delays and
the old artifact identity.

- [ ] **Step 2: Update the CLI and reports**

Rename the reference to
`01_reference_fixed-microdelay-master-envelope.wav`. Build it from all twelve
exact original layers using `0,1,3,4,5,7,8,9,11,12,14,15 ms`. Report the
fixed offsets, equal-power trim, master ADSR `25/180/0.88/320 ms`, shared group
gain, active RMS, peak, ceiling proportion, tonal/noise evidence, residuals,
and hashes.

Do not write isolated layers, solo files, long-delay references, mono
diagnostics, sweeps, or failed WAVs.

- [ ] **Step 3: Run deterministic CLI tests**

```bash
cargo test --test hybrid_subset_cli
cargo clippy --bin hybrid-subset-lab --test hybrid_subset_cli -- -D warnings
```

Expected: two test-mode batches are byte-identical except
`workstation-cost.txt`, contain exactly eight WAVs, and document only
micro-delayed master-enveloped composites.

- [ ] **Step 4: Commit**

```bash
git add src/bin/hybrid-subset-lab.rs tests/hybrid_subset_cli.rs
git commit -m "feat: render coherent hybrid composites"
```

### Task 3: Generate the only final artifact batch and update durable status

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Generate ignored: `artifacts/coherent-hybrid-composites/`

- [ ] **Step 1: Generate two fresh release batches**

```bash
cargo run --release --bin hybrid-subset-lab -- render <first-temp-directory>
cargo run --release --bin hybrid-subset-lab -- render <second-temp-directory>
```

Expected: both runs pass the engineering gate and are byte-identical except
the workstation timing file.

- [ ] **Step 2: Review and install only the new batch**

Confirm every retained WAV passes active level, ≤1% ceiling contact, tonal,
noise, residual, DC, jump, stereo/mono, finite, and common-tail rules. Copy
one verified batch to `artifacts/coherent-hybrid-composites/`.

Assert:

```bash
find artifacts -mindepth 1 -maxdepth 1 -type d -printf '%f\n'
```

Expected output:

```text
coherent-hybrid-composites
```

- [ ] **Step 3: Update durable documents**

Record the old sequential-delay batch as a timing/envelope interpretation
failure, the exact fixed offsets and master ADSR, final metrics, deterministic
hash, and open human listening verdict. Do not claim musical success.

- [ ] **Step 4: Commit**

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md
git commit -m "docs: record coherent hybrid composite gate"
```

### Task 4: Full verification and local integration

**Files:**
- Verify all tracked changes
- Inspect ignored `artifacts/coherent-hybrid-composites/`

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

Confirm the final artifact directory is ignored, contains exactly eight
passing WAVs, contains no rejected output, and is the only child under
`artifacts/`. Confirm the worktree is clean and no generated artifact is
tracked.

- [ ] **Step 3: Merge locally without pushing**

Fast-forward the verified feature branch into local `main`, rerun the complete
test suite on merged `main`, copy the verified ignored batch into the main
workspace, and remove the temporary worktree/branch.
