# Coupled Wire Envelope and Motion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Produce an exact Coupled Wire reference and three longer-envelope
developments with restrained high-only stereo motion and strict rejection
evidence.

**Architecture:** Preserve `StruckObject::new` as the immutable accepted path,
add a crate-private duration/loss configuration used only by a new isolated
`coupled_wire_motion` module, and keep all envelope/crossover/LFO state prepared
outside the sample path. A dedicated lab writes only passing deterministic
WAVs; production `Engine` remains untouched.

**Tech Stack:** Scalar Rust, prepared modal recurrences, prepared one-pole
crossover and sine/cosine LFO rotation, `assert_no_alloc`, `hound`, Cargo.

---

### Task 1: Preserve the accepted reference while allowing longer modal loss

**Files:**
- Modify: `src/struck_object.rs`

- [x] Add a failing unit test that renders the default Coupled Wire through
  both `StruckObject::new` and a crate-private configured constructor with
  `duration_ms = 1600` and `decay_scale = 1.0`, hashes both stereo streams, and
  requires exact equality.
- [x] Add a failing validation test requiring zero duration, non-finite decay,
  and non-positive decay scale to return `StruckError::InvalidConfiguration`.
- [x] Run:

```bash
cargo test --lib struck_object::tests::configured_reference_is_sample_identical
cargo test --lib struck_object::tests::configured_render_rejects_invalid_bounds
```

  Confirm RED because the configured constructor and error do not exist.
- [x] Implement:

```rust
pub(crate) fn configured(
    topology: StruckTopology,
    sample_rate: u32,
    duration_ms: u32,
    decay_scale: f32,
) -> Result<Self, StruckError>
```

  Make `new` delegate with `DURATION_MS` and `1.0`. Multiply only the declared
  mode decay seconds by `decay_scale`; do not alter ratios, detune, injection,
  coupling, excitation, pickup weights, DC control, or default boundary
  behavior.
- [x] Re-run all `struck_object::tests`, confirming exact reference equality,
  finite output, distinct topology hashes, decay evidence, and allocation-free
  sampling remain GREEN.
- [x] Commit:

```bash
git add src/struck_object.rs
git commit -m "refactor: configure Coupled Wire modal duration"
```

### Task 2: Add whole-sound envelopes and high-only zero-sum motion

**Files:**
- Create: `src/coupled_wire_motion.rs`
- Modify: `src/lib.rs`

- [x] Write failing contract tests requiring exactly these profiles and values:

```rust
Reference: duration 1600 ms
WarmHold: attack 200, decay 650, sustain 0.78, note_off 1900, release 1100
SlowOrbit: attack 200, decay 900, sustain 0.84, note_off 2100, release 1200,
           pan 2.6 Hz depth 0.10
FastOrbit: attack 200, decay 500, sustain 0.74, note_off 1800, release 1500,
           pan 6.2 Hz depth 0.055
```

- [x] Write failing behavior tests requiring:

```rust
assert!(rms_80_140ms > rms_0_60ms);
assert!(rms_160_220ms > rms_80_140ms);
assert!(release_late_rms < release_early_rms);
assert_eq!(last_stereo_frame, [0.0, 0.0]);
assert!(mono_difference_between_motion_on_and_off <= 1.0e-6);
```

  For every sample call, wrap `voice.sample()` in `assert_no_alloc`.
- [x] Run:

```bash
cargo test --lib coupled_wire_motion::tests::
```

  Confirm RED because the module does not exist.
- [x] Implement `CoupledMotionProfile`, immutable `CoupledMotionSpec`,
  `CoupledWireEnvelopeVoice`, prepared one-pole low/high split at `320 Hz`,
  and prepared sine/cosine rotation. Use `smoothstep(x) = x*x*(3-2*x)` for
  attack, decay, and release stage curves.
- [x] Construct developed voices with Coupled Wire, duration matching the
  profile, and modal decay scale `5.0`. Add pan as:

```rust
left += pan * high_mid;
right -= pan * high_mid;
```

  before applying the same master-envelope value to both channels. Preserve
  the original low-pass and side components; use no audio-rate nonlinear
  process.
- [x] Re-run the focused tests and all existing struck-object tests. Confirm
  GREEN and no reference hash change.
- [x] Commit:

```bash
git add src/coupled_wire_motion.rs src/lib.rs src/struck_object.rs
git commit -m "feat: shape Coupled Wire envelope motion"
```

### Task 3: Add shared presentation and rejection evidence

**Files:**
- Modify: `src/coupled_wire_motion.rs`

- [x] Write a failing test rendering the three developed previews, selecting
  one gain, and requiring every presented output to pass:

```rust
-14.0 <= total_rms_dbfs && total_rms_dbfs <= -10.0
ceiling_proportion <= 0.01
dc.abs() <= 0.002
mono_loss_db <= 1.0
tonal_pass_fraction == 1.0
noise_like == false
finite == true
returns_to_zero == true
```

- [x] Add a failing deliberately weak-render test that must contain
  `CoupledMotionRejection::TotalRms`.
- [x] Run the two focused tests and confirm RED because preview, shared-gain,
  evidence, and rejection APIs do not exist.
- [x] Implement `CoupledMotionPreview`, `CoupledMotionRender`,
  `CoupledMotionEvidence`, explicit rejection reasons, shared-gain selection,
  and the existing static `OUTPUT_CEILING`. The reference uses exact gain
  `4.43`; only the three developed profiles participate in the new shared-gain
  search.
- [x] Re-run all `coupled_wire_motion::tests` and Clippy for the library with
  warnings denied. Confirm GREEN.
- [x] Commit:

```bash
git add src/coupled_wire_motion.rs
git commit -m "feat: reject defective Coupled Wire developments"
```

### Task 4: Build the disposable listening lab

**Files:**
- Create: `src/bin/coupled-wire-envelope-lab.rs`
- Create: `tests/coupled_wire_envelope_cli.rs`

- [x] Write the CLI test first. Run two `render-test` generations and require
  byte-identical deterministic files except `workstation-cost.txt`, exactly:

```text
00_coupled_wire_reference.wav
01_coupled_wire_warm_hold.wav
02_coupled_wire_slow_orbit.wav
03_coupled_wire_fast_orbit.wav
```

  Require stereo 32-bit float 48 kHz WAVs with durations
  `1600/3000/3300/3300 ms`, no WAV for a rejected row, and README statements
  that the low body and mono sum stay fixed while only high-band stereo motion
  is introduced.
- [x] Run:

```bash
cargo test --test coupled_wire_envelope_cli
```

  Confirm RED because `CARGO_BIN_EXE_coupled-wire-envelope-lab` is absent.
- [x] Implement `render` and `render-test`, passing-only WAV writing,
  `manifest.tsv`, `envelopes.tsv`, `motion.tsv`, `metrics.tsv`,
  `rejections.tsv`, `hashes.tsv`, `generation-summary.tsv`, `README.md`, and
  `workstation-cost.txt`.
- [x] Re-run the CLI test, motion unit tests, existing struck-object tests, and
  focused Clippy with warnings denied. Confirm GREEN.
- [x] Commit:

```bash
git add src/bin/coupled-wire-envelope-lab.rs tests/coupled_wire_envelope_cli.rs
git commit -m "feat: present Coupled Wire envelope comparisons"
```

### Task 5: Generate, replace artifacts, document, and verify

**Files:**
- Modify: `docs/HANDOFF.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/COMPOSITE_MACHINE_RESEARCH.md`
- Generate ignored: `artifacts/coupled-wire-envelope-motion/`
- Remove ignored after replacement:
  `artifacts/moj-struck-objects/`

- [x] Build release and generate two independent 48 kHz batches in `mktemp -d`
  directories. Compare every deterministic file except
  `workstation-cost.txt`.
- [x] Inspect all rejection, RMS, ceiling, tonal, noise, mono, envelope, and
  final-zero evidence. If any developed profile fails, install no new batch.
- [x] Confirm the generated reference WAV is byte-identical to the previous
  `01_coupled_wire.wav`. Then move the prior artifact directory to Trash and
  install exactly one passing new directory.
- [x] Record the positive Coupled Wire verdict, exact envelope/motion
  contracts, metrics, aggregate SHA-256, and open human gate in all three
  durable documents.
- [x] Run:

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

- [x] Generate two more release batches, compare them to each other and the
  installed batch excluding workstation timing, and confirm artifact hygiene.
- [x] Commit documentation:

```bash
git add docs/HANDOFF.md docs/RESEARCH.md docs/COMPOSITE_MACHINE_RESEARCH.md
git commit -m "docs: record Coupled Wire envelope gate"
```

- [x] Use `superpowers:verification-before-completion` and
  `superpowers:finishing-a-development-branch`; fast-forward local `main`,
  copy the ignored passing batch, rerun all-target tests, remove the temporary
  branch/worktree, and do not push.
