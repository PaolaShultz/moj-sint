# Piano-Strike Envelope Audition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the rejected steady envelope audition with three short
piano-like monophonic renders whose common body decays to silence and whose
brief pitched strike source varies among the exact Cross, Spectral, and Dual
single-note mechanisms.

**Architecture:** Keep `envelope_audition` as the isolated research boundary.
Replace attack-time profiles with strike-source profiles, use one prepared
piecewise one-shot body envelope, and add one separately prepared exact-source
strike buffer selected by profile. Reuse the existing shared-gain and evidence
pipeline, then update the offline lab reports and deterministic CLI contract.

**Tech Stack:** Stable scalar Rust, existing Moj Sint hybrid renderers,
`assert_no_alloc`, `hound`, Cargo integration tests, TSV/Markdown reports.

---

## File Structure

- Modify `src/envelope_audition.rs`: strike profile contract, body/strike
  envelopes, prepared strike layer, rendering, evidence, and unit tests.
- Modify `src/bin/hybrid-subset-lab.rs`: piano-strike filenames, manifest,
  summaries, README, and rejection-aware WAV output.
- Modify `tests/envelope_audition_cli.rs`: deterministic three-WAV listening
  contract and 800 ms format assertions.
- Modify `docs/HANDOFF.md`: reject the previous batch and record the new
  listening gate.
- Modify `docs/RESEARCH.md`: durable envelope and strike conclusions.
- Modify `docs/COMPOSITE_MACHINE_RESEARCH.md`: detailed source, envelope, and
  measurement checkpoint.

### Task 1: Define the exact piano-strike profile contract

**Files:**

- Modify: `src/envelope_audition.rs`

- [ ] **Step 1: Replace the old profile test with a failing strike contract**

Add a test that requires:

```rust
assert_eq!(
    StrikeProfile::ALL.map(StrikeProfile::strike_layer),
    [
        OriginalLayer::CrossSingle,
        OriginalLayer::SpectralSingle,
        OriginalLayer::DualSingle,
    ]
);
assert_eq!(
    StrikeProfile::ALL.map(StrikeProfile::strike_ms),
    [16, 28, 42]
);
assert_eq!(DURATION_MS, 800);
assert_eq!(BODY_ATTACK_MS, 2);
assert_eq!(BODY_FAST_DECAY_MS, 60);
assert_eq!(BODY_SILENT_FROM_MS, 700);
assert_eq!(STRIKE_ATTACK_MS, 1);
assert_eq!(STRIKE_MIX, 0.30);
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```bash
cargo test envelope_audition::tests::profiles_vary_exact_short_strike_sources
```

Expected: compilation fails because `StrikeProfile` and the new constants do
not exist.

- [ ] **Step 3: Implement the minimal profile API**

Replace `AttackProfile` with:

```rust
pub enum StrikeProfile {
    Cross,
    Spectral,
    Dual,
}
```

Define `ALL`, `strike_layer`, `strike_ms`, `slug`, `label`, and `filename`
methods. Define the constants required by the test. Preserve
`MONOPHONIC_LAYERS`, `LAYER_OFFSETS_MS`, `MIN_TOTAL_RMS`, and
`MAX_TOTAL_RMS`.

- [ ] **Step 4: Run the focused test and verify GREEN**

Run:

```bash
cargo test envelope_audition::tests::profiles_vary_exact_short_strike_sources
```

Expected: one passing test and zero failures.

- [ ] **Step 5: Commit**

```bash
git add src/envelope_audition.rs
git commit -m "feat: define piano strike audition profiles"
```

### Task 2: Render one shared one-shot body and varied exact-source strikes

**Files:**

- Modify: `src/envelope_audition.rs`

- [ ] **Step 1: Write failing envelope and allocation tests**

Add tests which instantiate each profile at 8 kHz and assert:

```rust
assert_eq!(mixer.duration_frames(), 6_400);
assert_eq!(mixer.body_envelope_at_ms(0), 0.0);
assert!(mixer.body_envelope_at_ms(2) >= 0.99);
assert!((mixer.body_envelope_at_ms(62) - 0.28).abs() < 0.02);
assert!(mixer.body_envelope_at_ms(300) < 0.28);
assert_eq!(mixer.body_envelope_at_ms(700), 0.0);
assert_eq!(mixer.body_envelope_at_ms(799), 0.0);
```

For each profile, sample the full render inside `assert_no_alloc`, require
finite values, require exact-zero final frames, and assert that the prepared
strike source and length match the profile.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```bash
cargo test envelope_audition::tests::piano_body_has_no_flat_sustain
cargo test envelope_audition::tests::complete_piano_strike_sum_is_allocation_free
```

Expected: compilation fails because the one-shot envelope inspection and
prepared strike behavior do not exist.

- [ ] **Step 3: Implement the one-shot body and strike envelopes**

Create a prepared body envelope with:

```text
0..2 ms: linear 0.0 -> 1.0
2..62 ms: squared easing 1.0 -> 0.28
62..700 ms: squared easing 0.28 -> 0.0
700..800 ms: exact zero
```

Prepare the three common body sources exactly as before. Also render the
profile's exact single-note source once into a prepared strike buffer. Apply a
`1 ms` strike rise and squared decay to exact zero at `16/28/42 ms`, multiply
by `STRIKE_MIX`, and add it before the shared presentation gain and static
ceiling. All buffer preparation remains outside `sample`.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test envelope_audition::tests::
```

Expected: all envelope-audition unit tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/envelope_audition.rs
git commit -m "feat: render piano body with varied pitched strikes"
```

### Task 3: Preserve whole-file RMS rejection and engineering gates

**Files:**

- Modify: `src/envelope_audition.rs`

- [ ] **Step 1: Write or update failing gain and rejection tests**

Require `preview_all(48_000)` to contain three distinct strike profiles. Select
one gain, render all three, and assert each total RMS is inside:

```rust
MIN_TOTAL_RMS..=MAX_TOTAL_RMS
```

Require every render to have no rejection reasons. Retain the deliberately weak
`0.01` gain test and assert `EnvelopeRejection::TotalRms`.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test envelope_audition::tests::shared_gain_keeps_piano_strikes_inside_the_total_rms_gate
```

Expected: failure until preview/render/evidence types use `StrikeProfile` and
the short renders satisfy the shared-gain policy.

- [ ] **Step 3: Adapt the preview/render/evidence pipeline**

Change `EnvelopePreview` and `EnvelopeRender` to store `StrikeProfile`. Keep
whole-file RMS over all `800 ms`, active-region metrics, tonal projection,
noise classification, DC/jump/stereo/mono limits, sparse ceiling contact, and
return-to-zero checks. Do not relax a threshold to make a profile pass.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```bash
cargo test envelope_audition::tests::
```

Expected: all envelope-audition tests pass with zero warnings.

- [ ] **Step 5: Commit**

```bash
git add src/envelope_audition.rs
git commit -m "feat: gate piano strikes by whole-file rms"
```

### Task 4: Replace the disposable listening presentation

**Files:**

- Modify: `src/bin/hybrid-subset-lab.rs`
- Modify: `tests/envelope_audition_cli.rs`

- [ ] **Step 1: Update the deterministic CLI test first**

Require exactly:

```text
01_piano_cross_strike.wav
02_piano_spectral_strike.wav
03_piano_dual_strike.wav
```

Require stereo float WAVs with exactly `800 ms` duration. Require README text
for `piano-like one-shot`, `no flat sustain`, `pitched D2 strike`, and
`whole-file RMS`; reject `2.4 seconds`, `chord`, and `progression`. Continue
checking that rejected rows never receive WAVs and two runs are deterministic.

- [ ] **Step 2: Run the CLI test and verify RED**

Run:

```bash
cargo test --test envelope_audition_cli
```

Expected: failure showing the old attack filenames and 2.4-second contract.

- [ ] **Step 3: Update the lab reports and output contract**

Change manifest fields to include strike source, strike length, body attack,
fast decay, silence boundary, duration, shared gain, total RMS, and status.
Change generation summary and README to describe the common one-shot body and
the three varied exact-source strikes. Write WAVs only for passing rows.

- [ ] **Step 4: Run the CLI and focused unit tests and verify GREEN**

Run:

```bash
cargo test --test envelope_audition_cli
cargo test envelope_audition::tests::
cargo clippy --bin hybrid-subset-lab --test envelope_audition_cli -- -D warnings
```

Expected: all pass with zero warnings.

- [ ] **Step 5: Commit**

```bash
git add src/bin/hybrid-subset-lab.rs tests/envelope_audition_cli.rs
git commit -m "feat: present piano strike envelope comparisons"
```

### Task 5: Generate only the final passing batch and record the checkpoint

**Files:**

- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Generate ignored: `artifacts/piano-strike-envelope-composites/`

- [ ] **Step 1: Generate two fresh release batches outside the repository**

Run:

```bash
piano_temp_a=$(mktemp -d /tmp/moj-sint-piano-a.XXXXXX)
piano_temp_b=$(mktemp -d /tmp/moj-sint-piano-b.XXXXXX)
cargo run --release --bin hybrid-subset-lab -- render "$piano_temp_a"
cargo run --release --bin hybrid-subset-lab -- render "$piano_temp_b"
diff -rq --exclude=workstation-cost.txt "$piano_temp_a" "$piano_temp_b"
```

Expected: three passing WAVs per directory, no rejected WAVs, and no
deterministic differences.

- [ ] **Step 2: Apply the artifact gate**

Verify every present row is at least `-14 dBFS`, every rejection row with
status `reject` has no WAV, there are exactly three WAVs, all reports exist,
and noise-like flags are false. If any check fails, retain no listening
directory and fix the cause rather than lowering thresholds.

- [ ] **Step 3: Install only one verified batch**

Copy one verified release directory to
`artifacts/piano-strike-envelope-composites/`. Confirm it is ignored and the
artifacts tree contains no other directory.

- [ ] **Step 4: Update durable documentation**

Record the prior 0.58-sustain batch as rejected and deleted, the new exact
envelope and strike contracts, final metrics, deterministic aggregate hash,
open human listening gate, and the fact that nothing is integrated into
`Engine`.

- [ ] **Step 5: Run complete verification**

Run:

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

Expected: all commands exit zero; `cargo deny` may retain only the already
accepted `winnow` duplicate warning.

- [ ] **Step 6: Commit durable documentation**

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md
git commit -m "docs: record piano strike envelope audition"
```

- [ ] **Step 7: Finish locally**

Use `superpowers:verification-before-completion` and
`superpowers:finishing-a-development-branch`. Fast-forward to local `main`,
copy the ignored verified artifact batch to the main checkout, rerun all-target
tests on the merged tree, remove the feature worktree and branch, and do not
push.
