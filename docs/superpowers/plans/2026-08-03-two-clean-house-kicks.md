# Two Clean House Kicks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and present two deterministic, clean, structurally distinct kick voices: a tight envelope-driven PM House Impact and a long linearly coupled-resonator Long Pressure.

**Architecture:** Add an isolated `clean_kick` research module with separate callback-safe voices and an offline measurement/selection layer. A dedicated CLI writes exactly two solo WAVs and two 124 BPM repeated-context WAVs plus deterministic reports; production Engine, presets, models, JACK, ALSA, and SHR-DAW remain unchanged.

**Tech Stack:** Stable scalar Rust, existing 4096-sample sine table, fixed callback state, `assert_no_alloc`, `hound`, windowed-sinc 8x reference resampling, Cargo tests.

---

## File map

- Create `src/clean_kick/mod.rs`: public types, configuration, rendering, and scheduling.
- Create `src/clean_kick/house.rs`: House Impact source.
- Create `src/clean_kick/pressure.rs`: Long Pressure source.
- Create `src/clean_kick/measurements.rs`: evidence, ablations, selection, and rejection.
- Modify `src/lib.rs`: export the isolated module.
- Create `src/bin/clean-kick-lab.rs`: four-WAV/report writer.
- Create `tests/clean_kick_contract.rs` and `tests/clean_kick_cli.rs`.
- Modify canonical docs, knowledge notes, and this plan with verified results.

### Task 1: Establish House Impact test-first

**Files:** Create `src/clean_kick/mod.rs`, `src/clean_kick/house.rs`, and `tests/clean_kick_contract.rs`; modify `src/lib.rs`.

- [ ] **Step 1: Write the failing public contract**

```rust
use assert_no_alloc::assert_no_alloc;
use moj_sint::clean_kick::{KickTopology, PreparedKick, SAMPLE_RATE};

#[test]
fn house_impact_is_deterministic_finite_and_allocation_free() {
    let mut a = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    let mut b = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    assert_no_alloc(|| {
        a.trigger();
        b.trigger();
        for _ in 0..SAMPLE_RATE {
            let left = a.sample();
            let right = b.sample();
            assert_eq!(left.to_bits(), right.to_bits());
            assert!(left.is_finite());
        }
    });
}
```

- [ ] **Step 2: Run RED**

Run `cargo test --test clean_kick_contract house_impact_is_deterministic_finite_and_allocation_free`.
Expected: missing module and public types.

- [ ] **Step 3: Implement the public boundary and source**

```rust
pub const SAMPLE_RATE: u32 = 48_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KickTopology { HouseImpact, LongPressure }

pub enum PreparedKick { House(house::HouseImpact), Pressure(pressure::LongPressure) }

impl PreparedKick {
    pub fn trigger(&mut self) {
        match self { Self::House(v) => v.trigger(), Self::Pressure(v) => v.trigger() }
    }
    #[inline]
    pub fn sample(&mut self) -> f32 {
        match self { Self::House(v) => v.sample(), Self::Pressure(v) => v.sample() }
    }
}
```

Define exact invalid-rate/config/render-length/no-passing-coordinate errors.
Implement House Impact with `SineTable::lookup`, wrapped cycle phases,
geometric 156 -> 52 Hz motion over 55 ms, 2:1 modifier, 0.9-radian index
decaying over 45 ms, 1 ms zero-start attack, 320 ms -60 dB decay, static gain,
and no nonlinear or ceiling stage. Triggering resets fixed state only.

- [ ] **Step 4: Run GREEN and add unit contracts**

Run the focused test and `cargo test clean_kick::house::tests --lib`. Prove
pitch settlement, monotonic contours, bounded phase, maximum jump `< 0.10`,
exact eventual silence, and allocation-free trigger/sample/retrigger.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/clean_kick tests/clean_kick_contract.rs
git commit -m "Add clean House Impact kick source"
```

### Task 2: Implement Long Pressure test-first

**Files:** Create `src/clean_kick/pressure.rs`; modify `src/clean_kick/mod.rs` and `tests/clean_kick_contract.rs`.

- [ ] **Step 1: Add the failing distinct/stable contract**

```rust
#[test]
fn long_pressure_is_distinct_stable_and_allocation_free() {
    let mut house = PreparedKick::new(KickTopology::HouseImpact, SAMPLE_RATE).unwrap();
    let mut long = PreparedKick::new(KickTopology::LongPressure, SAMPLE_RATE).unwrap();
    assert_no_alloc(|| {
        house.trigger();
        long.trigger();
        let mut difference = 0.0_f64;
        for _ in 0..SAMPLE_RATE {
            let a = house.sample();
            let b = long.sample();
            assert!(b.is_finite());
            difference += f64::from((a - b).abs());
        }
        assert!(difference > 100.0);
    });
}
```

- [ ] **Step 2: Run RED**

Run the focused test. Expected: Long Pressure is absent or silent.

- [ ] **Step 3: Implement two prepared linear resonators**

```rust
#[derive(Clone, Copy, Default)]
struct Resonator { y1: f32, y2: f32 }

impl Resonator {
    #[inline]
    fn sample(&mut self, input: f32, a1: f32, a2: f32, gain: f32) -> f32 {
        let y = gain * input + a1 * self.y1 - a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}
```

Prepare `a1 = 2*r*cos(2*pi*f/fs)` and `a2 = r*r` outside sampling. Use a 1 ms
raised-cosine excitation, impact 112 -> 76 Hz/115 ms decay, body 58 -> 48 Hz
over 280 ms/760 ms decay, and 100 ms linear impact-to-body coupling. Apply one
static gain and clear only inactive state below `1e-8`.

- [ ] **Step 4: Run GREEN and stability contracts**

Run `cargo test clean_kick::pressure::tests --lib` and the focused integration
test. Prove pole radii `< 1`, finite state, decreasing post-excitation energy,
44-54 Hz settlement by 350 ms, exact silence, and allocation-free retrigger.

- [ ] **Step 5: Commit**

```bash
git add src/clean_kick tests/clean_kick_contract.rs
git commit -m "Add clean Long Pressure kick source"
```

### Task 3: Add evidence, ablations, and ordered selection

**Files:** Create `src/clean_kick/measurements.rs`; modify `src/clean_kick/mod.rs` and `tests/clean_kick_contract.rs`.

- [ ] **Step 1: Add failing evidence coverage**

```rust
#[test]
fn selected_voices_pass_every_engineering_gate() {
    for topology in [KickTopology::HouseImpact, KickTopology::LongPressure] {
        let config = select(topology, SAMPLE_RATE).unwrap();
        let evidence = evaluate(&render_solo(config, SAMPLE_RATE).unwrap()).unwrap();
        assert!(evidence.rejection_reasons().is_empty(), "{evidence:#?}");
        assert!((-6.0..=-1.0).contains(&evidence.metrics.sample_peak_dbfs));
        assert!(evidence.metrics.true_peak_dbfs <= -1.0);
        assert_eq!(evidence.metrics.ceiling_contacts, 0);
        assert!(evidence.metrics.absolute_dc < 1.0e-5);
        assert!(evidence.metrics.maximum_jump < 0.10);
        assert!(evidence.high_rate_residual_db <= -60.0);
    }
}
```

- [ ] **Step 2: Run RED**

Run the focused test. Expected: render and evidence APIs are absent.

- [ ] **Step 3: Implement metrics and fixed-order rejection**

Add exact render/metrics/evidence types and rejection variants for finiteness,
sample/true peak, ceiling contact, DC, jump, decay/tail, pitch settlement,
resonator stability/energy, ablation, and high-rate residual. Independently
render 8x and use the existing windowed-sinc/alignment method; require residual
`<= -60 dB` relative to active reference RMS.

- [ ] **Step 4: Implement causal selection**

Select House candidates in ascending modifier-index order and Pressure
candidates in ascending coupling order. House mute requires >= 6 dB first-80-ms
100-800 Hz difference and <= 1 dB late-low-body change. Pressure mute requires
>= 4 dB first-140-ms 70-250 Hz difference and retained 44-54 Hz late energy.
Sweeps never become listening files.

- [ ] **Step 5: Run and commit**

```bash
cargo test clean_kick --lib
cargo test --test clean_kick_contract
cargo test --all-targets --all-features
git add src/clean_kick tests/clean_kick_contract.rs
git commit -m "Validate clean kick listening candidates"
```

Expected: all normal tests pass; historical ignored renderers remain ignored.

### Task 4: Build the exact four-file lab test-first

**Files:** Create `src/bin/clean-kick-lab.rs` and `tests/clean_kick_cli.rs`; modify `src/clean_kick/mod.rs`.

- [ ] **Step 1: Add failing CLI inventory coverage**

```rust
#[test]
fn lab_writes_four_deterministic_clean_kick_wavs() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    run_lab(first.path());
    run_lab(second.path());
    assert_eq!(deterministic_files(first.path()), deterministic_files(second.path()));
    assert_eq!(wav_names(first.path()), [
        "01_house-impact_solo.wav", "02_long-pressure_solo.wav",
        "03_house-impact_124bpm.wav", "04_long-pressure_124bpm.wav",
    ]);
}
```

- [ ] **Step 2: Run RED**

Run `cargo test --test clean_kick_cli`. Expected: binary absent.

- [ ] **Step 3: Implement scheduling, WAVs, and reports**

Add `render_repeated(config, 48_000, 124, 4)` with quarter-note triggers in
one persistent voice. CLI modes `render`/`render-test` evaluate both voices
before writing 32-bit float, 48 kHz dual-mono WAVs. Write exactly README,
settings/metrics/hashes/generation-summary/workstation-cost reports and four
WAVs. README names every forbidden processor and leaves musical value open.

- [ ] **Step 4: Run and commit**

```bash
cargo test --test clean_kick_cli
cargo test --test clean_kick_contract
git add src/bin/clean-kick-lab.rs src/clean_kick tests/clean_kick_cli.rs
git commit -m "Add clean kick listening lab"
```

Expected: two temporary generations match except workstation timing.

### Task 5: Generate, document, verify, and hand off

**Files:** Modify `docs/HANDOFF.md`, `docs/RESEARCH.md`, both Moj Sint knowledge notes, and this plan.

- [ ] **Step 1: Generate two fresh release batches**

Use two `mktemp -d` destinations and compare every file except
`workstation-cost.txt`. Then replace only `artifacts/two-clean-house-kicks/`
and render there; preserve all other artifact directories.

- [ ] **Step 2: Record measured evidence**

Document selected coordinates, levels, DC/jump/decay, ablations, high-rate
residuals, inventory/hash, tests, and open listening questions. Do not infer
massive, clean-sounding, house-ready, or accepted from metrics.

- [ ] **Step 3: Run final verification**

```bash
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo audit
cargo deny check
git diff --check
/home/shome/Documents/knowledge/.zk/validate.sh
/home/shome/Documents/knowledge/.zk/tests/validate-test.sh
```

Expected: all pass. Historical ignored renderers remain ignored.

- [ ] **Step 4: Verify boundaries and finish**

Confirm no production engine/model/preset/host file changed, no artifact is
tracked, exactly four WAVs and declared reports exist, final release generation
matches, and no JACK/ALSA/hardware path was touched. Mark this plan complete,
commit durable source/docs, and hand off clickable WAV paths for listening.
