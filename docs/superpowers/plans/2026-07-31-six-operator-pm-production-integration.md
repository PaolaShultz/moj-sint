# Six-Operator PM Production Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge the completed six-operator PM research engine into Moj Sint `main`, promote its six sounds to a live `Six-Op PM` model, and expose that model and its twelve controls through SHR-DAW.

**Architecture:** Moj Sint remains one external host whose strict preset selects either `Model D` or `Six-Op PM`; its production engine dispatches through the existing fixed model enum. SHR-DAW remains one managed melodic-engine owner and extends only Moj Sint model discovery, route identity, control labels, persistence, and playback presentation.

**Tech Stack:** Rust 2024, Serde/TOML, fixed-capacity scalar DSP, JACK/ALSA host boundary, Cargo tests, Clippy, cargo-audit, cargo-deny, Git worktrees.

---

### Task 1: Integrate the verified research branch with the model-identity base

**Files:**
- Merge: `feature/six-operator-pm` with Moj Sint `main`
- Resolve if needed: `docs/HANDOFF.md`
- Resolve if needed: `src/lib.rs`

- [ ] **Step 1: Verify both worktrees and branch tips**

Run:

```bash
git -C /home/shome/p/moj-sint status --short --branch
git -C /home/shome/.config/superpowers/worktrees/moj-sint/six-operator-pm status --short --branch
git -C /home/shome/p/moj-sint merge-base --is-ancestor 41fb374 feature/six-operator-pm
```

Expected: both worktrees are clean and the ancestry check exits zero.

- [ ] **Step 2: Merge current `main` into the feature worktree**

Run:

```bash
git merge main
```

Expected: a merge commit retaining both the schema-4 model seam and the complete six-op history. Resolve documentation additively; do not remove either model-identity or six-op evidence.

- [ ] **Step 3: Verify the merged research baseline**

Run:

```bash
cargo fmt --check
cargo test --all-targets --all-features
```

Expected: formatting passes and the pre-production merged baseline has zero test failures.

### Task 2: Add strict schema-5 model-specific presets

**Files:**
- Modify: `src/preset.rs`
- Modify: `src/control.rs`
- Modify: `docs/PRESET_SCHEMA.md`

- [ ] **Step 1: Write failing schema tests**

Add tests that require:

```rust
assert_eq!(Preset::parse(SIX_OP_VALID)?.model, SynthesisModelId::SixOpPm);
assert_eq!(Preset::parse(SIX_OP_VALID)?.model_patch, ModelPatchId::SixOpPm(SixOpPatchId::BellMetal));
assert!(Preset::parse(SIX_OP_WITH_MODEL_D_PATCH).is_err());
assert_eq!(Preset::parse(V4_MODEL_D)?.model, SynthesisModelId::ModelD);
```

Also require all twelve Six-Op PM macro values to parse only as finite normalized values.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test preset::tests::parses_strict_version_five_six_op_preset -- --exact
```

Expected: compilation or assertion failure because `SixOpPm`, `SixOpPatchId`, and schema 5 do not exist.

- [ ] **Step 3: Implement the model-specific preset boundary**

Add these production identities:

```rust
pub enum SynthesisModelId { ModelD, SixOpPm }
pub enum SixOpPatchId {
    BellMetal,
    FracturedMetal,
    ElectricPianoMallet,
    GlassWood,
    BrassBass,
    MechanicalStab,
}
pub enum ModelPatchId {
    ModelD(ModelDPatchId),
    SixOpPm(SixOpPatchId),
}
```

Use strict schema-5 structs selected by `model`. Keep schema 1-4 migration to `ModelD`. Store model controls internally as twelve normalized values and expose model-specific lookup without weakening unknown-field rejection.

- [ ] **Step 4: Run schema tests and verify GREEN**

Run:

```bash
cargo test preset::tests
```

Expected: all preset tests pass.

- [ ] **Step 5: Commit the preset boundary**

Run:

```bash
git add src/preset.rs src/control.rs docs/PRESET_SCHEMA.md
git diff --cached --check
git commit -m "feat: add six-op PM preset identity"
```

### Task 3: Add the live Six-Op PM voice adapter and controls

**Files:**
- Modify: `src/dsp/six_op_pm/mod.rs`
- Modify: `src/dsp/six_op_pm/operator.rs`
- Create: `src/six_op_pm/live.rs`
- Modify: `src/six_op_pm.rs`
- Modify: `src/synthesis_model.rs`
- Modify: `src/engine.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing adapter tests**

Add tests requiring a prepared live voice to:

```rust
let mut voice = LiveSixOpVoice::new(48_000.0, SixOpPatchId::BellMetal)?;
voice.set_live_controls([0.5; 8]);
voice.note_on(60, 0.8);
let first = render(&mut voice, 4096);
voice.reset();
voice.set_live_controls([0.5; 8]);
voice.note_on(60, 0.8);
assert_eq!(first, render(&mut voice, 4096));
```

Add one focused test per control showing a non-neutral value changes only its declared prepared destination. Add an `assert_no_alloc` render/control test and note-off/reset tests.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```bash
cargo test six_op_pm::live::tests
```

Expected: failure because the live adapter does not exist.

- [ ] **Step 3: Implement prepared continuous control laws**

Implement eight smoothed normalized controls:

```rust
pub enum SixOpMacro {
    Index, Ratio, Feedback, Decay,
    Balance, KeyScale, Velocity, Motion,
}
```

Prepare bounds and coefficients outside sampling. Keep algorithm topology preset-owned. Share one immutable sine table across preallocated voices. Retune and retrigger without allocation. Forward note-off to operator and pitch envelopes. Keep finite guards and diagnostics.

- [ ] **Step 4: Dispatch through `VoiceModel`**

Make `VoiceModel::new` accept `ModelPatchId` and construct either `ModelDVoice` or `LiveSixOpVoice`. Extend `set_live_controls`, `note_on`, `note_off`, `sample`, and `reset` exhaustively. Update `Engine` to create the selected model and forward note-off before the outer ADSR release.

- [ ] **Step 5: Run adapter and engine tests and verify GREEN**

Run:

```bash
cargo test six_op_pm::live::tests
cargo test engine::tests
```

Expected: all focused tests pass with no allocation assertion failures.

- [ ] **Step 6: Commit the live model**

Run:

```bash
git add src/dsp/six_op_pm src/six_op_pm src/six_op_pm.rs src/synthesis_model.rs src/engine.rs src/lib.rs
git diff --cached --check
git commit -m "feat: add live six-op PM synthesis model"
```

### Task 4: Promote the six authored sounds to factory presets

**Files:**
- Create: `presets/08-six-op-bell-metal.mojsint`
- Create: `presets/09-six-op-fractured-metal.mojsint`
- Create: `presets/10-six-op-electric-piano-mallet.mojsint`
- Create: `presets/11-six-op-glass-wood.mojsint`
- Create: `presets/12-six-op-brass-bass.mojsint`
- Create: `presets/13-six-op-mechanical-stab.mojsint`
- Modify: `src/preset.rs`
- Modify: `tests/compact_contract.rs`

- [ ] **Step 1: Write failing factory-catalog tests**

Require exactly thirteen factory presets, seven Model D plus six Six-Op PM, unique names, correct model/patch identity, twelve normalized values, and valid voice/gain bounds.

- [ ] **Step 2: Run the factory test and verify RED**

Run:

```bash
cargo test preset::tests::factory_presets_cover_both_models -- --exact
```

Expected: failure because the six production preset files do not exist.

- [ ] **Step 3: Add strict schema-5 factory presets**

Each file uses:

```toml
schema_version = 5
model = "six_op_pm"
six_op_patch = "bell_metal"
```

with its authored fixed output gain, production voice count, neutral eight Six-Op PM macros, and usable ADSR. Substitute the correct patch identifier and truthful authored coordinates for each file.

- [ ] **Step 4: Verify factory presets and neutral reproduction**

Run:

```bash
cargo test preset::tests::factory_presets_cover_both_models -- --exact
cargo test six_op_pm::live::tests::neutral_factory_starts_reproduce_authored_single_notes -- --exact
```

Expected: both tests pass.

- [ ] **Step 5: Commit factory presets**

Run:

```bash
git add presets src/preset.rs tests/compact_contract.rs
git diff --cached --check
git commit -m "feat: add six-op PM factory presets"
```

### Task 5: Complete Moj Sint documentation and verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/PRESET_SCHEMA.md`
- Modify: `docs/RESEARCH.md`

- [ ] **Step 1: Update current documentation and package version**

Record the second synthesis model, six factory presets, exact controls, schema-5 migration, clean-room boundary, and verification evidence. Remove statements that production integration has not started. Advance Moj Sint from `0.2.2` to `0.3.0` because this adds a new playable synthesis model and preset wire format.

- [ ] **Step 2: Run the full Moj Sint gate**

Run:

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --locked
cargo audit
cargo deny check
git diff --check
```

Also run the existing deterministic six-op generation comparison and focused alias/error suite. Expected: every command exits zero; only the documented accepted duplicate-version warning may remain in cargo-deny.

- [ ] **Step 3: Commit verified Moj Sint integration**

Run:

```bash
git add Cargo.toml Cargo.lock docs
git diff --cached --check
git commit -m "docs: record live six-op PM integration"
```

### Task 6: Create the isolated SHR-DAW integration worktree

**Files:**
- Worktree: `/home/shome/.config/superpowers/worktrees/shr-daw/moj-six-op-pm`

- [ ] **Step 1: Verify SHR-DAW `main` is clean and current**

Run:

```bash
git -C /home/shome/p/shr-daw fetch --prune origin
git -C /home/shome/p/shr-daw status --short --branch
git -C /home/shome/p/shr-daw merge --ff-only origin/main
```

Expected: clean `main`, equal to `origin/main`.

- [ ] **Step 2: Create the global worktree**

Run:

```bash
git -C /home/shome/p/shr-daw worktree add /home/shome/.config/superpowers/worktrees/shr-daw/moj-six-op-pm -b feature/moj-six-op-pm
```

Expected: clean feature worktree based on current SHR-DAW `main`.

- [ ] **Step 3: Record the approved repository compile gate**

The user approved the written design whose test-first verification section explicitly requires the combined formatting, tests, Clippy, and locked release-build gate. That approval authorizes the repository's combined build-and-test pass. It does not authorize JACK, synth, MIDI, playback, recording, or physical-hardware actions.

### Task 7: Extend SHR-DAW model discovery, controls, and persistence

**Files:**
- Modify: `src/preset.rs`
- Modify: `src/control.rs`
- Modify: `src/engine.rs`
- Modify: `src/recording.rs`
- Modify: `src/ui.rs`

- [ ] **Step 1: Write failing SHR-DAW tests before production edits**

Add tests requiring:

```rust
assert_eq!(MojModel::SixOpPm.stable_id(), "six_op_pm");
assert_eq!(preset.route_id(), "six_op_pm/Bell Metal");
assert_eq!(moj_controls(MojModel::SixOpPm)[0].name, "Index");
assert_eq!(moj_controls(MojModel::SixOpPm)[11].name, "Release");
```

Add strict schema-5 discovery/rejection, twelve-value parsing, Project/Idea/FT2 identity, recording round-trip, legacy Model D route, LOAD command, RESET, pickup, rollback, and one-managed-engine tests.

- [ ] **Step 2: Verify RED under the approved combined build-and-test scope**

Run:

```bash
cargo test preset::tests::discovers_schema_five_six_op_pm -- --exact
```

Expected: compilation or assertion failure because `MojModel::SixOpPm` is missing.

- [ ] **Step 3: Implement model-specific discovery and controls**

Add `MojModel::SixOpPm`, stable identity, label, and `MOJ_SIX_OP_PM_CONTROLS`. Extend strict Moj schema-5 parsing by model, returning normalized CC 20-31 values. Preserve schema 1-4 Model D migration and unqualified legacy Model D resolution.

- [ ] **Step 4: Extend existing consumers exhaustively**

Make engine state, Playback controls, RESET, pickup, Ideas, Projects, FT2 routes, recording metadata, and UI presentation consume the selected model's control table and model-qualified identity. Do not add another backend, process, JACK route, ALSA port, config block, screen, or controller bank.

- [ ] **Step 5: Run focused SHR-DAW tests and verify GREEN**

Run:

```bash
cargo test preset::tests::discovers_schema_five_six_op_pm -- --exact
cargo test control::tests
cargo test recording::tests
cargo test ui::tests::moj -- --nocapture
```

Expected: focused tests pass. If the UI filter matches no exact test name, run the newly added exact model-specific UI tests individually.

- [ ] **Step 6: Commit SHR-DAW code integration**

Run:

```bash
git add src/preset.rs src/control.rs src/engine.rs src/recording.rs src/ui.rs
git diff --cached --check
git commit -m "feat: add Six-Op PM Moj Sint model"
```

### Task 8: Complete SHR-DAW documentation and authorized verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `docs/CONFIGURATION.md`
- Modify: `docs/CONTROLLER_INTERFACE.md`
- Modify: `docs/INSTALLATION.md`
- Modify: `docs/TRACKER.md`
- Modify: `docs/WORKSPACE_HANDOFF.md`
- Modify generated docs only through the owning generator

- [ ] **Step 1: Update version and focused documentation**

Advance SHR-DAW from `0.4.7` to `0.4.8`. Record Moj Sint's two selectable
models, schema 5, model-qualified route IDs, Six-Op PM controls, six presets,
single-process ownership, and compatibility.

- [ ] **Step 2: Run the authorized SHR-DAW gate**

Under the approved combined scope, run:

```bash
rustc -vV
cargo fmt --check
cargo check --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --locked
cargo build --locked --release
python3 scripts/generate-docs-site.py --write
python3 scripts/generate-docs-site.py --check
git diff --check
```

Expected: every command exits zero, the normal test suite has zero failures,
the generated site is stable on its immediate check, and both DEV and REL
artifacts build. Do not start JACK, synths, MIDI, playback, recording, or
physical hardware.

- [ ] **Step 3: Commit verified SHR-DAW documentation**

Run:

```bash
git add Cargo.toml Cargo.lock docs
git diff --cached --check
git commit -m "docs: document Six-Op PM integration"
```

### Task 9: Merge, synchronize knowledge, and publish

**Files:**
- Modify: `/home/shome/Documents/knowledge/Moj Sint/01 Current State.md`
- Modify if routed by index: `/home/shome/Documents/knowledge/Moj Sint/04 Next Actions And Open Questions.md`

- [ ] **Step 1: Merge Moj Sint feature work into `main`**

Run from `/home/shome/p/moj-sint`:

```bash
git status --short --branch
git merge --no-ff feature/six-operator-pm
```

Expected: `main` contains the design, complete research history, and verified live integration.

- [ ] **Step 2: Merge SHR-DAW feature work into `main`**

Run from `/home/shome/p/shr-daw`:

```bash
git status --short --branch
git merge --no-ff feature/moj-six-op-pm
```

Expected: `main` contains the paired discovery/control/persistence integration.

- [ ] **Step 3: Update and validate shared project knowledge**

Update concise current state and next action with source links and verified evidence, then run:

```bash
/home/shome/Documents/knowledge/.zk/validate.sh
/home/shome/Documents/knowledge/.zk/tests/validate-test.sh
```

Expected: both validators exit zero.

- [ ] **Step 4: Verify publication targets**

Run in both repositories:

```bash
git status --short --branch
git remote -v
git log --oneline origin/main..main
```

Expected: clean `main`, intended public `origin`, and only the reviewed task commits ahead.

- [ ] **Step 5: Push both repositories non-interactively**

Run:

```bash
GIT_TERMINAL_PROMPT=0 git -C /home/shome/p/moj-sint push origin main
GIT_TERMINAL_PROMPT=0 git -C /home/shome/p/shr-daw push origin main
```

Expected: both pushes succeed and local `main` equals `origin/main`.
