# Kick Comparison With Snares Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render and verify the approved 15-WAV comparison of two Moj Sint kicks and three SHR Drums kicks, including controlled and native snare contexts plus raw and level-matched reels.

**Architecture:** A temporary Rust crate outside every repository reads the verified Moj Sint float WAVs, loads the three factory `.shrkit` packages through the real SHR Drums engine, schedules deterministic note events, assembles presentations, measures them, and writes two fresh candidate batches. After byte comparison, one candidate is atomically moved into Moj Sint's ignored artifact path and the temporary crate/builds are trashed.

**Tech Stack:** Rust 2024, `hound` 3.5, local `shr-drums` path dependency, standard-library SHA-256 invocation, existing factory manifests and WAV assets.

---

## File map

- Create temporarily: `$comparison_tmp/Cargo.toml` — isolated renderer dependencies.
- Create temporarily: `$comparison_tmp/src/main.rs` — source decoding, SHR Drums event rendering, assembly, measurement, reports, and CLI.
- Create temporarily: `$comparison_tmp/tests/contracts.rs` — artifact and failure-boundary tests.
- Create: `artifacts/kick-comparison-with-snares/` — ignored disposable batch with exactly 15 WAVs and seven evidence files.
- Do not modify production source, Cargo manifests, factory packages, or installed data in any repository.

### Task 1: Establish the temporary renderer contract

**Files:**
- Create temporarily: `$comparison_tmp/Cargo.toml`
- Create temporarily: `$comparison_tmp/src/main.rs`
- Create temporarily: `$comparison_tmp/tests/contracts.rs`

- [ ] **Step 1: Create a uniquely named temporary crate root**

Run:

```bash
comparison_tmp=$(mktemp -d /tmp/moj-sint-kick-comparison.XXXXXX)
printf '%s\n' "$comparison_tmp"
```

Expected: one absolute directory matching
`/tmp/moj-sint-kick-comparison.*`; retain the exact value for every later task.

- [ ] **Step 2: Write the crate manifest**

Use `apply_patch` to create:

```toml
[package]
name = "kick-comparison-renderer"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
hound = "3.5"
shr-drums = { path = "/home/shome/p/shr-drums/crates/shr-drums" }
```

- [ ] **Step 3: Write failing integration tests for the CLI contract**

`tests/contracts.rs` must run the compiled binary with `render-test`, the Moj
Sint artifact root, the SHR-DAW kit root, and a fresh destination. Tests must
require exactly 15 `.wav` names from the approved design, exactly seven report
names, rejection of a nonempty destination, stereo float32 48 kHz WAV headers,
five equal solo frame counts, eight equal pattern frame counts, finite samples,
absolute raw peak below 1.0, and five matched-reel segment peaks within 0.01 dB.

The command shape fixed by the test is:

```text
kick-comparison-renderer <render|render-test> \
  <two-clean-house-kicks-directory> <factory-kit-directory> <output-directory>
```

- [ ] **Step 4: Write the minimum CLI argument parser**

`src/main.rs` must accept only the exact four-argument shape above, reject any
other command or path shape with exit status 1, refuse a nonempty output, and
delegate accepted input to:

```rust
fn render_batch(
    moj_sources: &Path,
    kit_root: &Path,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>>;
```

The initial function returns `Err("renderer not implemented".into())` so the
contract test reaches a deliberate RED state.

- [ ] **Step 5: Run the contract test and verify RED**

Run:

```bash
cargo test --manifest-path "$comparison_tmp/Cargo.toml" --test contracts -- --nocapture
```

Expected: test failure containing `renderer not implemented`; no repository
file changes and no artifact directory created.

### Task 2: Implement deterministic source rendering

**Files:**
- Modify temporarily: `$comparison_tmp/src/main.rs`
- Modify temporarily: `$comparison_tmp/tests/contracts.rs`

- [ ] **Step 1: Add focused source tests**

Add tests that require:

```rust
#[derive(Clone)]
struct Audio {
    frames: Vec<[f32; 2]>,
}

#[derive(Clone, Copy)]
struct TimedEvent {
    frame: usize,
    note: u8,
    velocity: u8,
}
```

`read_float_stereo` must reject non-48-kHz, non-stereo, non-float32, nonfinite,
or malformed WAVs. `render_kit` must reject an unexpected manifest kit ID and
render note 36 and note 38 deterministically from fresh engines. A repeated
render from the same input must produce byte-identical `f32::to_bits()` values.

- [ ] **Step 2: Run source tests and verify RED**

Run:

```bash
cargo test --manifest-path "$comparison_tmp/Cargo.toml" source_ -- --nocapture
```

Expected: compile failure because `Audio`, `TimedEvent`, `read_float_stereo`,
and `render_kit` do not yet exist.

- [ ] **Step 3: Implement strict WAV decoding**

Implement `read_float_stereo(path)` with `hound::WavReader`. Require
`channels == 2`, `sample_rate == 48_000`, `bits_per_sample == 32`, and
`sample_format == Float`; collect consecutive left/right samples into
`Vec<[f32; 2]>`, reject an odd sample count, and reject any nonfinite sample.

- [ ] **Step 4: Implement engine-authentic kit rendering**

Implement `render_kit(package, expected_id, events, frame_count)` as follows:

```rust
let prepared = shr_drums::load_package(
    package,
    shr_drums::ProjectKey::default(),
    &shr_drums::KitTuning::default(),
)?;
if prepared.manifest.kit_id != expected_id {
    return Err(format!("expected kit {expected_id}").into());
}
let (sender, receiver) = shr_drums::event_queue();
let mut engine = shr_drums::DrumEngine::new(48_000, prepared, receiver)?;
```

Sort and validate events by frame. For every frame, push all events at that
frame as `DrumEvent::NoteOn`, process one `StereoFrame`, and copy it into the
result. Reject queue errors, nonfinite output, and any request beyond the fixed
frame count. This deliberately uses one engine instance for native patterns
and separate fresh instances for controlled kick-only and snare-only sources.

- [ ] **Step 5: Run source tests and verify GREEN**

Run:

```bash
cargo test --manifest-path "$comparison_tmp/Cargo.toml" source_ -- --nocapture
```

Expected: all focused source tests pass with no JACK, ALSA, or SHR-DAW process.

### Task 3: Assemble the 15 presentations and evidence

**Files:**
- Modify temporarily: `$comparison_tmp/src/main.rs`
- Modify temporarily: `$comparison_tmp/tests/contracts.rs`

- [ ] **Step 1: Add failing assembly and measurement tests**

Tests must cover `pad_or_fail`, `linear_sum`, `peak`, `rms`, `absolute_mean`,
`maximum_jump`, `dbfs`, `write_wav`, and reel assembly. Require exact silence
padding, frame-preserving linear sums, rejection at absolute peak `>= 1.0`,
correct RMS for known stereo fixtures, and matched gains that attenuate every
source to the quietest solo peak without boosting that reference.

- [ ] **Step 2: Run assembly tests and verify RED**

Run:

```bash
cargo test --manifest-path "$comparison_tmp/Cargo.toml" assembly_ -- --nocapture
```

Expected: compile failure naming the missing assembly helpers.

- [ ] **Step 3: Implement timing and source inventory**

Use constants `SAMPLE_RATE=48_000`, `BPM=124`, `BARS=4`, `VELOCITY=110`, and
notes 36/38. Solo triggers occur at frame 12,000. Quarter-note trigger frame N
is `N * SAMPLE_RATE * 60 / BPM` for N 0 through 15; snare events use N values
1, 3, 5, 7, 9, 11, 13, and 15. The initial common duration is three seconds
for solos and four bars plus two seconds for patterns. Extend every member of a
group together if the longest source has not produced 250 ms of terminal
digital silence.

Load the two Moj Sint solo and repeated files named by their existing artifact
manifest. Move a solo's first nonzero sample to frame 12,000 without changing
its samples. Preserve repeated samples at frame zero and zero-pad them to the
common pattern duration. Render the three old kick solos, three old kick-only
patterns, one Electronic House snare-only pattern, and three native patterns
through fresh SHR Drums engines.

- [ ] **Step 4: Implement controlled mixes, native files, and reels**

Create the five common-snare files by linearly summing each kick-only pattern
with the exact Electronic House snare-only source. Fail if any sum reaches
absolute 1.0. Write native patterns directly from their single-engine renders.
Create the raw reel from five three-second solo segments plus one second of
digital silence after each segment. Create the matched reel with static gains
`quietest_peak / source_peak`; require every gain in `(0, 1]` and segment peaks
within 0.01 dB after application.

- [ ] **Step 5: Implement deterministic reports**

Write all 15 WAVs with float32 stereo headers and the seven report files named
in the design. Use stable inventory order, tabs, LF newlines, six-decimal
floating formatting, and absolute source paths only in `workstation-cost.txt`.
Create `hashes.tsv` last by invoking `sha256sum` on every deterministic file
except `hashes.tsv` itself and `workstation-cost.txt`, then include the WAV and
report hashes in sorted filename order. Record all three package-manifest
SHA-256 values and the current Moj Sint/SHR Drums Git revisions in
`generation-summary.tsv`.

- [ ] **Step 6: Run all temporary-crate tests and verify GREEN**

Run:

```bash
cargo test --manifest-path "$comparison_tmp/Cargo.toml" --all-targets -- --nocapture
```

Expected: all contract, source, assembly, inventory, and failure-boundary tests
pass; no repository working-tree changes.

### Task 4: Generate twice and install the disposable batch

**Files:**
- Create temporarily: `$comparison_tmp/out-a/`
- Create temporarily: `$comparison_tmp/out-b/`
- Create: `/home/shome/p/moj-sint/artifacts/kick-comparison-with-snares/`

- [ ] **Step 1: Render two fresh release batches**

Run:

```bash
cargo run --release --manifest-path "$comparison_tmp/Cargo.toml" -- \
  render-test \
  /home/shome/p/moj-sint/artifacts/two-clean-house-kicks \
  /home/shome/p/shr-daw/kits \
  "$comparison_tmp/out-a"
cargo run --release --manifest-path "$comparison_tmp/Cargo.toml" -- \
  render-test \
  /home/shome/p/moj-sint/artifacts/two-clean-house-kicks \
  /home/shome/p/shr-daw/kits \
  "$comparison_tmp/out-b"
```

Expected: both exit 0 and each output contains exactly 15 WAVs plus the seven
declared reports.

- [ ] **Step 2: Compare deterministic content**

Run:

```bash
diff -qr --exclude=workstation-cost.txt "$comparison_tmp/out-a" "$comparison_tmp/out-b"
```

Expected: no output and exit 0.

- [ ] **Step 3: Refuse replacement of an existing batch**

If `/home/shome/p/moj-sint/artifacts/kick-comparison-with-snares` exists and is
nonempty, stop and inspect it; do not overwrite it. Otherwise move `out-a` to
that exact final path with `mv --`. Do not retain `out-b`.

- [ ] **Step 4: Verify the final artifact contract independently**

Run shell checks that count 15 WAVs and seven reports, parse every WAV header
with the renderer's `render-test` validation mode, confirm `metrics.tsv` has 15
data rows, confirm no raw peak is `>= 0 dBFS`, and confirm every path is ignored
by `git check-ignore -q`.

### Task 5: Cleanup and final repository proof

**Files:**
- Remove safely: `$comparison_tmp/`

- [ ] **Step 1: Preserve only the canonical batch**

Validate that `$comparison_tmp` matches
`/tmp/moj-sint-kick-comparison.*`, then move that exact directory to trash with
`gio trash -- "$comparison_tmp"`. Do not delete or trash any repository path.

- [ ] **Step 2: Verify all owning repositories are clean**

Run:

```bash
git -C /home/shome/p/moj-sint status --short --branch
git -C /home/shome/p/shr-daw status --short --branch
git -C /home/shome/p/shr-drums status --short --branch
git -C /home/shome/p/moj-sint check-ignore -v \
  artifacts/kick-comparison-with-snares
```

Expected: all working trees are clean apart from the already committed design
and plan history, and the final batch is ignored by Moj Sint's artifact rule.

- [ ] **Step 3: Hand off listening evidence without accepting a sound**

Report the raw solo and pattern peak/RMS/headroom table, provide the exact
absolute batch path, state whether any common mix approached the ceiling, and
give the recommended listening order. Do not claim that a kick is massive,
clean, balanced, or selected before the user listens.
