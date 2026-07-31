# Six-Operator PM Listening Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an isolated clean-room six-operator phase-modulation engine and a deterministic six-file listening gate containing three category-authentic/original Moj Sint topology pairs.

**Architecture:** A new `dsp::six_op_pm` directory owns fixed-capacity graph, envelope, operator, and voice state. A top-level `six_op_pm` research module owns six authored patches, three fixed scores, offline rendering, analysis, and acceptance; `six-op-pm-lab` owns filesystem presentation. Production `Engine`, schema 3, the twelve controls, JACK/ALSA, and SHR-DAW remain untouched.

**Tech Stack:** Stable scalar Rust, fixed arrays, `thiserror`, existing `hound`, `assert_no_alloc`, and existing research measurement helpers.

**Execution prerequisite:** Invoke `superpowers:using-git-worktrees` and create an isolated worktree under `/home/shome/.config/superpowers/worktrees/moj-sint/` before source changes. Do not push unless explicitly requested.

---

## File map

- Create `src/dsp/six_op_pm/algorithm.rs`: graph facts, 32 connectivity records, validation, prepared order.
- Create `src/dsp/six_op_pm/envelope.rs`: deterministic four-rate/four-level envelopes.
- Create `src/dsp/six_op_pm/operator.rs`: frequency modes, scaling, sine lookup, operator state.
- Create `src/dsp/six_op_pm/mod.rs`: patch validation, voice, delayed feedback, LFO, errors.
- Modify `src/dsp/mod.rs`: expose isolated DSP.
- Create `src/six_op_pm.rs`: patches, scores, ensemble, rendering, measurements, acceptance.
- Modify `src/lib.rs`: expose offline research module.
- Create `src/bin/six-op-pm-lab.rs`: render WAVs and reports.
- Create `tests/six_op_pm_cli.rs`: deterministic end-to-end contract.
- Modify `src/research.rs`: expose the existing fitted-residual helper without changing it.
- Modify `docs/RESEARCH.md`, `docs/ARCHITECTURE.md`, `docs/HANDOFF.md`, and Moj Sint knowledge notes after verification.

## Task 1: Validated 32-graph catalog

**Files:**
- Create: `src/dsp/six_op_pm/algorithm.rs`
- Create: `src/dsp/six_op_pm/mod.rs`
- Modify: `src/dsp/mod.rs:1-5`

- [ ] **Step 1: Write catalog tests first**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_graphs_prepare_and_have_unique_connectivity() {
        assert_eq!(CLASSIC_ALGORITHMS.len(), 32);
        let mut fingerprints = Vec::new();
        for spec in CLASSIC_ALGORITHMS {
            let prepared = PreparedAlgorithm::new(spec).unwrap();
            assert_ne!(prepared.carrier_mask(), 0);
            fingerprints.push(spec.fingerprint());
        }
        fingerprints.sort_unstable();
        fingerprints.dedup();
        assert_eq!(fingerprints.len(), 32);
    }

    #[test]
    fn group_feedback_is_removed_before_topological_sort() {
        let prepared = PreparedAlgorithm::new(CLASSIC_ALGORITHMS[3]).unwrap();
        assert_eq!(prepared.feedback(), Edge::one_based(4, 6));
        assert!(prepared.position(6) < prepared.position(5));
        assert!(prepared.position(5) < prepared.position(4));
    }

    #[test]
    fn undeclared_cycle_and_carrierless_graph_are_rejected() {
        const CYCLE: AlgorithmSpec = AlgorithmSpec::new(
            99, &[Edge::one_based(1, 2), Edge::one_based(2, 1)],
            carrier_mask(&[1]), Edge::one_based(6, 6));
        const NO_CARRIER: AlgorithmSpec = AlgorithmSpec::new(
            100, &[], 0, Edge::one_based(6, 6));
        assert_eq!(PreparedAlgorithm::new(CYCLE), Err(AlgorithmError::Cycle));
        assert_eq!(PreparedAlgorithm::new(NO_CARRIER), Err(AlgorithmError::Carrierless));
    }
}
```

- [ ] **Step 2: Run red**

Run: `cargo test dsp::six_op_pm::algorithm::tests --all-features`

Expected: compilation fails because graph types and catalog do not exist.

- [ ] **Step 3: Implement graph types and preparation**

```rust
use thiserror::Error;
pub const OPERATOR_COUNT: usize = 6;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge { pub source: u8, pub target: u8 }
impl Edge {
    pub const fn one_based(source: u8, target: u8) -> Self {
        Self { source: source - 1, target: target - 1 }
    }
}
pub const fn carrier_mask(operators: &[u8]) -> u8 {
    let mut mask = 0; let mut index = 0;
    while index < operators.len() { mask |= 1 << (operators[index] - 1); index += 1; }
    mask
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AlgorithmSpec {
    pub id: u8, pub edges: &'static [Edge], pub carriers: u8, pub feedback: Edge,
}
impl AlgorithmSpec {
    pub const fn new(id: u8, edges: &'static [Edge], carriers: u8, feedback: Edge) -> Self {
        Self { id, edges, carriers, feedback }
    }
    pub fn fingerprint(self) -> u64 {
        let mut hash = u64::from(self.carriers)
            | (u64::from(self.feedback.source) << 8) | (u64::from(self.feedback.target) << 12);
        for edge in self.edges { hash = hash.rotate_left(7) ^ (u64::from(edge.source) << 4) ^ u64::from(edge.target); }
        hash
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AlgorithmError {
    #[error("invalid operator index")] InvalidOperator,
    #[error("duplicate edge")] DuplicateEdge,
    #[error("no carrier")] Carrierless,
    #[error("undeclared cycle")] Cycle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedAlgorithm {
    incoming: [[u8; 6]; 6], incoming_count: [u8; 6], order: [u8; 6],
    carriers: u8, feedback: Edge, output_gain: f32,
}
```

`PreparedAlgorithm::new` must validate every index, duplicate edge, and carrier mask; derive a six-node Kahn topological order from ordinary edges only; reject any remaining cycle; prepare incoming source arrays; and set `output_gain = 0.75 / sqrt(carrier_count)`. Add getters `carrier_mask`, `feedback`, `position`, `evaluation_order`, `incoming`, `is_carrier`, and `output_gain`. No heap allocation is needed in preparation.

- [ ] **Step 4: Add the full primary-source-derived catalog**

Transcribe graph connectivity from the official Yamaha PLG100-DX Owner's Manual algorithm chart on PDF pages 28-29: `https://usa.yamaha.com/files/download/other_assets/1/320951/PLG100DXE.pdf`. Record that source in `docs/RESEARCH.md`; do not copy the chart artwork or prose. The feedback edge is delayed and therefore excluded from `edges`:

```rust
const fn a(id: u8, edges: &'static [Edge], carriers: &[u8], feedback: Edge) -> AlgorithmSpec {
    AlgorithmSpec::new(id, edges, carrier_mask(carriers), feedback)
}
const fn e(source: u8, target: u8) -> Edge { Edge::one_based(source, target) }
pub const CLASSIC_ALGORITHMS: [AlgorithmSpec; 32] = [
    a(1,&[e(6,5),e(5,4),e(4,3),e(2,1)],&[1,3],e(6,6)),
    a(2,&[e(6,5),e(5,4),e(4,3),e(2,1)],&[1,3],e(2,2)),
    a(3,&[e(3,2),e(2,1),e(6,5),e(5,4)],&[1,4],e(6,6)),
    a(4,&[e(3,2),e(2,1),e(6,5),e(5,4)],&[1,4],e(4,6)),
    a(5,&[e(2,1),e(4,3),e(6,5)],&[1,3,5],e(6,6)),
    a(6,&[e(2,1),e(4,3),e(6,5)],&[1,3,5],e(5,6)),
    a(7,&[e(2,1),e(4,3),e(6,5),e(5,3)],&[1,3],e(6,6)),
    a(8,&[e(2,1),e(4,3),e(6,5),e(5,3)],&[1,3],e(4,4)),
    a(9,&[e(2,1),e(4,3),e(6,5),e(5,3)],&[1,3],e(2,2)),
    a(10,&[e(3,2),e(2,1),e(5,4),e(6,4),e(4,1)],&[1],e(3,3)),
    a(11,&[e(3,2),e(2,1),e(5,4),e(6,4),e(4,1)],&[1],e(6,6)),
    a(12,&[e(2,1),e(4,3),e(5,3),e(6,3),e(3,1)],&[1],e(2,2)),
    a(13,&[e(4,3),e(5,3),e(6,3),e(3,1),e(2,1)],&[1],e(6,6)),
    a(14,&[e(5,2),e(2,1),e(6,4),e(4,3)],&[1,3],e(6,6)),
    a(15,&[e(5,2),e(2,1),e(6,4),e(4,3)],&[1,3],e(2,2)),
    a(16,&[e(2,1),e(4,3),e(3,1),e(6,5),e(5,1)],&[1],e(6,6)),
    a(17,&[e(2,1),e(4,3),e(3,1),e(6,5),e(5,1)],&[1],e(2,2)),
    a(18,&[e(2,1),e(3,1),e(6,5),e(5,4),e(4,1)],&[1],e(3,3)),
    a(19,&[e(3,2),e(2,1),e(6,4),e(6,5)],&[1,4,5],e(6,6)),
    a(20,&[e(3,1),e(5,2),e(6,4)],&[1,2,4],e(3,3)),
    a(21,&[e(3,1),e(6,4)],&[1,2,4,5],e(3,3)),
    a(22,&[e(2,1),e(6,3),e(6,4),e(6,5)],&[1,3,4,5],e(6,6)),
    a(23,&[e(3,1),e(6,2),e(6,4),e(6,5)],&[1,2,4,5],e(6,6)),
    a(24,&[e(6,1),e(6,2),e(6,3),e(6,4),e(6,5)],&[1,2,3,4,5],e(6,6)),
    a(25,&[e(6,4)],&[1,2,3,4,5],e(6,6)),
    a(26,&[e(3,2),e(5,4),e(6,4)],&[1,2,4],e(6,6)),
    a(27,&[e(3,2),e(5,4),e(6,4)],&[1,2,4],e(3,3)),
    a(28,&[e(2,1),e(5,4),e(4,3)],&[1,3,6],e(5,5)),
    a(29,&[e(4,3),e(6,5)],&[1,2,3,5],e(6,6)),
    a(30,&[e(5,4),e(4,3)],&[1,2,3,6],e(5,5)),
    a(31,&[e(6,5)],&[1,2,3,4,5],e(6,6)),
    a(32,&[],&[1,2,3,4,5,6],e(6,6)),
];
```

- [ ] **Step 5: Run green and commit**

Run: `cargo test dsp::six_op_pm::algorithm::tests --all-features && cargo fmt --check`

Expected: 3 passed, 0 failed; formatting exits 0.

```bash
git add src/dsp/mod.rs src/dsp/six_op_pm
git commit -m "feat: add validated six-operator graph catalog"
```

## Task 2: Four-stage envelopes and prepared operators

**Files:**
- Create: `src/dsp/six_op_pm/envelope.rs`
- Create: `src/dsp/six_op_pm/operator.rs`
- Modify: `src/dsp/six_op_pm/mod.rs`

- [ ] **Step 1: Add failing primitive tests**

```rust
#[test]
fn envelope_reaches_three_key_on_levels_then_releases() {
    let spec = EnvelopeSpec::new([0.01,0.02,0.03,0.04],[1.0,0.7,0.5,0.0]).unwrap();
    let mut envelope = Envelope::new(spec, 1_000.0).unwrap();
    envelope.note_on();
    for _ in 0..60 { envelope.advance(); }
    assert!((envelope.level() - 0.5).abs() < 1.0e-5);
    envelope.note_off();
    for _ in 0..40 { envelope.advance(); }
    assert!(envelope.is_idle());
}

#[test]
fn operator_prepares_ratio_fixed_key_and_velocity_outside_sample_path() {
    let env = EnvelopeSpec::new([0.01;4],[1.0,0.8,0.6,0.0]).unwrap();
    let ratio = OperatorSpec::ratio(2.0,0.8,env,0.5,KeyboardScaling::flat(),0.0).unwrap();
    let fixed = OperatorSpec::fixed(777.0,0.5,env,0.0,KeyboardScaling::flat(),0.0).unwrap();
    let a = PreparedOperator::new(ratio,48_000.0,69,0.5).unwrap();
    let b = PreparedOperator::new(fixed,48_000.0,36,1.0).unwrap();
    assert!((a.frequency_hz()-880.0).abs()<0.01);
    assert!((b.frequency_hz()-777.0).abs()<0.01);
    assert!(a.prepared_level()<0.8);
}
```

- [ ] **Step 2: Run red**

Run: `cargo test dsp::six_op_pm --all-features`

Expected: compilation fails because primitive types are absent.

- [ ] **Step 3: Implement the exact primitive contracts**

`EnvelopeSpec { seconds: [f32;4], levels: [f32;4] }` validates positive finite seconds and levels in `0..=1`. `Envelope` starts at `levels[3]`, linearly traverses `L1/L2/L3` over `R1/R2/R3`, holds L3, and reaches L4 over R4 after note-off. It stores target, step, remaining samples, and an enum stage; `advance` returns squared level for perceptual gain. Expose `note_on`, `note_off`, `advance`, `level`, `is_idle`, and `reset`.

Implement these operator types:

```rust
pub enum FrequencyMode { Ratio(f32), Fixed(f32) }
pub struct KeyboardScaling { pub break_note:u8, pub left_db_per_octave:f32, pub right_db_per_octave:f32 }
pub struct OperatorSpec {
    pub frequency:FrequencyMode, pub output_level:f32, pub envelope:EnvelopeSpec,
    pub velocity_sensitivity:f32, pub scaling:KeyboardScaling, pub detune_cents:f32,
}
pub struct PreparedOperator {
    phase:f32, initial_phase:f32, increment:f32, frequency_hz:f32,
    prepared_level:f32, envelope:Envelope, sine_table:Arc<[f32;4096]>,
}
```

`OperatorSpec::new` rejects frequency outside `(0,20_000]`, level/sensitivity outside `0..=1`, scaling outside `-24..=24 dB/octave`, detune outside `-50..=50 cents`, and all non-finite input. `PreparedOperator::new` computes note frequency, detune, key gain, velocity gain, increment, and envelope. Define `SineTable::new()` with `std::array::from_fn` and `sin` outside the sample path; the ensemble creates one `Arc<[f32;4096]>` and prepared operators clone it only during construction. Linear interpolation wraps at the table boundary. `sample(phase_modulation_cycles, pitch_multiplier)` only reads the table, interpolates, advances phase by `increment * pitch_multiplier`, advances the envelope, and multiplies. Expose note/reset and measurement getters. Voice creation and destruction remain outside the callback; no `Arc` clone/drop occurs in a sample or event path.

- [ ] **Step 4: Run green and commit**

Run: `cargo test dsp::six_op_pm --all-features && cargo fmt --check`

Expected: all graph/envelope/operator tests pass.

```bash
git add src/dsp/six_op_pm
git commit -m "feat: add prepared six-operator primitives"
```

## Task 3: Allocation-free six-operator voice

**Files:**
- Modify: `src/dsp/six_op_pm/mod.rs`

- [ ] **Step 1: Add failing voice tests**

Test a two-operator fixture for carrier and sideband amplitude, graph-4 delayed feedback for correct previous-sample ownership, bounded/resettable pitch envelope and LFO, deterministic reset, note-off idle, finite bounded output at maximum declared feedback, rapid note events, and `assert_no_alloc` around 48,000 calls to `sample`.

- [ ] **Step 2: Run red**

Run: `cargo test dsp::six_op_pm::tests --all-features`

Expected: compilation fails because patch/voice types are absent.

- [ ] **Step 3: Implement patch and voice types**

```rust
pub const MAX_PM_CYCLES:f32=4.0;
pub const MAX_FEEDBACK_CYCLES:f32=2.0;
pub struct PitchEnvelopeSpec { pub seconds:[f32;4], pub semitones:[f32;4] }
pub struct LfoSpec { pub rate_hz:f32, pub pitch_depth_cents:f32 }
pub struct SixOpPatch {
    pub algorithm:usize, pub operators:[OperatorSpec;6],
    pub pitch_envelope:PitchEnvelopeSpec, pub lfo:LfoSpec,
    pub feedback:f32, pub output_gain:f32,
}
pub struct SixOpVoice {
    algorithm:PreparedAlgorithm, operators:[PreparedOperator;6], outputs:[f32;6],
    feedback_sample:f32, feedback_amount:f32,
    pitch_envelope:PitchEnvelope,
    lfo_sine:f32, lfo_cosine:f32, lfo_rotation_sine:f32, lfo_rotation_cosine:f32,
    lfo_pitch_depth_ratio:f32, output_gain:f32, non_finite_seen:bool, clamp_contacts:u64,
}
```

`PitchEnvelope` is a separate four-stage envelope because its values are not gains. In `SixOpVoice::new`, validate finite semitone levels within `-24..=24`, convert the four levels once with `2.0_f32.powf(semitones / 12.0)`, and prepare linear multiplier segments. Also prepare the LFO rotation and `lfo_pitch_depth_ratio = 2.0_f32.powf(pitch_depth_cents / 1200.0) - 1.0` once. In `sample`, advance the pitch envelope and quadrature LFO, form `pitch_multiplier = pitch_envelope.advance() * (1.0 + lfo_sine * lfo_pitch_depth_ratio)`, and pass it to every operator. Then evaluate operators in prepared order, sum ordinary incoming outputs at `MAX_PM_CYCLES`, add the previous feedback-source sample only at the feedback target, update feedback after all operators, sum carriers, and apply fixed gains. An unexpected non-finite result becomes silence for that sample and latches `non_finite_seen`; an emergency output clamp counts contacts. Tests and the listening gate require no non-finite results and zero contacts. No allocation, I/O, locks, logging, clocks, formatting, panic, `sin`, `cos`, or `powf` occurs in `sample`.

- [ ] **Step 4: Run green and commit**

Run: `cargo test dsp::six_op_pm --all-features`

Expected: spectral, feedback, reset, bounds, event, and allocation tests pass.

```bash
git add src/dsp/six_op_pm
git commit -m "feat: add allocation-free six-operator PM voice"
```

## Task 4: Six patches, three scores, and offline ensemble

**Files:**
- Create: `src/six_op_pm.rs`
- Modify: `src/lib.rs:1-25`

- [ ] **Step 1: Add failing inventory tests**

```rust
#[test]
fn gate_has_three_topology_pairs_and_six_files() {
    assert_eq!(ListeningPatch::ALL.map(ListeningPatch::pair),[0,0,1,1,2,2]);
    assert_eq!(ListeningPatch::ALL.map(|id|id.patch().algorithm),[0,0,4,4,9,9]);
    for pair in ListeningPatch::ALL.chunks_exact(2) {
        assert_eq!(pair[0].score(),pair[1].score());
        assert_eq!(pair[0].pair_gain(),pair[1].pair_gain());
    }
}
```

- [ ] **Step 2: Run red**

Run: `cargo test six_op_pm::tests --all-features`

Expected: compilation fails because the module is absent.

- [ ] **Step 3: Define exact inventory**

Use enum variants `BellMetal`, `FracturedMetal`, `ElectricPianoMallet`, `GlassWood`, `BrassBass`, and `MechanicalStab`. Filenames are exactly:

```text
01_bell-metal_reference.wav
02_fractured-metal_original.wav
03_electric-piano-mallet_reference.wav
04_glass-wood_original.wav
05_brass-bass_reference.wav
06_mechanical-stab_original.wav
```

Use algorithm indices `0`, `4`, and `9`. Define `ScoreAction::{On{note,velocity},Off{note}}` and sample-accurate events. Bell score: MIDI 60/72/48 at `0.00/1.20/2.45 s`. Mallet score: MIDI 60, then chord 60/64/67 at `1.15 s`. Brass score: MIDI 36/43/48/36 at `0.00/0.72/1.42/2.00 s`. Each note has an explicit off event; durations are `4.8/4.0/3.8 s`. Use four fixed offline voices and deterministic oldest-voice stealing.

- [ ] **Step 4: Author exact patch parameter tables**

Each operator tuple is `(ratio, level, attack, decay1, decay2, release, L1, L2, sustain, velocity, detune_cents)`:

```text
BellMetal: (1,.78,.003,.18,1.8,2.4,1,.62,.18,.25,0); (1.414,.68,.002,.09,.55,1.1,1,.48,.05,.55,0); (1,.58,.002,.24,2.4,2.8,1,.70,.22,.18,0); (2,.66,.001,.08,.70,1.2,1,.42,.03,.50,0); (1.414,.62,.001,.06,.42,.9,1,.35,.02,.45,0); (2.71,.52,.001,.04,.28,.7,1,.24,0,.40,0). Feedback .06.
FracturedMetal: (1,.74,.002,.12,1.2,1.7,1,.50,.12,.30,-3); (1.731,.72,.001,.05,.38,.8,1,.30,.01,.65,2); (.5,.54,.003,.20,1.7,2.1,1,.58,.16,.20,4); (2.347,.70,.001,.04,.32,.7,1,.28,.01,.60,-2); (1.219,.68,.001,.035,.24,.55,1,.20,0,.55,3); (3.913,.58,.001,.025,.18,.45,1,.12,0,.50,-4). Feedback .16.
ElectricPianoMallet: (1,.72,.004,.16,1.1,1.5,1,.72,.28,.45,0); (1,.52,.001,.055,.34,.65,1,.32,.01,.75,0); (1,.42,.003,.20,1.25,1.6,1,.68,.24,.35,-2); (14,.22,.001,.025,.16,.35,1,.16,0,.85,0); (2,.28,.002,.12,.80,1.1,1,.48,.12,.30,2); (3,.34,.001,.04,.24,.5,1,.22,0,.70,0). Feedback .03.
GlassWood: (1,.66,.003,.11,.72,1,1,.62,.18,.55,0); (2.71,.58,.001,.035,.22,.42,1,.18,0,.80,-2); (.5,.38,.002,.16,.90,1.3,1,.52,.14,.45,3); (7.07,.40,.001,.022,.14,.30,1,.12,0,.80,0); (1.5,.32,.002,.09,.58,.85,1,.38,.08,.50,-3); (4.13,.46,.001,.03,.20,.40,1,.15,0,.75,2). Feedback .08.
BrassBass: (1,.76,.012,.13,.50,.24,1,.84,.70,.20,0); (1,.56,.010,.10,.34,.20,1,.72,.52,.35,0); (1,.34,.008,.08,.28,.18,1,.64,.44,.30,0); (2,.48,.006,.09,.30,.20,1,.65,.42,.30,0); (1,.38,.005,.07,.24,.16,1,.58,.36,.28,-2); (3,.30,.004,.06,.20,.14,1,.52,.30,.24,2). Feedback .22.
MechanicalStab: (.5,.72,.002,.055,.22,.16,1,.54,.08,.25,0); (1.25,.66,.001,.035,.14,.11,1,.32,.01,.55,2); (2.01,.54,.001,.025,.10,.09,1,.22,0,.50,-2); (1.5,.62,.001,.030,.12,.10,1,.28,.01,.50,1); (2.75,.54,.001,.022,.09,.08,1,.20,0,.45,-1); (5.03,.44,.001,.018,.07,.07,1,.14,0,.40,3). Feedback .34.
```

Initial pair gains are `.72/.68/.74`. Use flat carrier scaling, `-1.5 dB/octave` above MIDI 72 for bright modulators, and `+1 dB/octave` below MIDI 48 for brass modulators. Subtle pitch envelopes are at most `0.8 semitone`; original variants may use at most twice the paired reference excursion. LFO is at most `6.7 Hz/5 cents`.

- [ ] **Step 5: Implement ensemble/render and run green**

Use `[Option<ResearchVoice>;4]`, deterministic note ownership, dual-mono interleaving, and no allocations inside the frame loop. Test four-note capacity, note-off, reset, finite output, and exact repeat hashes. Preallocate the output buffer, then put the fixed-event frame loop for four prepared voices inside `assert_no_alloc` so the complete render path—not only one operator—is covered.

Run: `cargo test six_op_pm --all-features`

Expected: inventory, score, ensemble, and render tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/six_op_pm.rs
git commit -m "feat: author six-operator PM listening pairs"
```

## Task 5: Measurements and acceptance

**Files:**
- Modify: `src/six_op_pm.rs`
- Modify: `src/research.rs`

- [ ] **Step 1: Expose and test existing residual fitting**

Change only `fn fitted_residual_db` to `pub fn fitted_residual_db`. Add regressions requiring identical and half-gain-scaled inputs to fit below `-200 dB`.

- [ ] **Step 2: Add failing acceptance tests**

For every patch require finite samples, peak `<=0.95`, zero clamp contacts, absolute DC `<=0.0002`, maximum jump `<=0.25`, the declared pitch rule, alias rows passing at MIDI 36/60/84, and within-pair active RMS difference `<=3 dB`. Require every report row to contain finite crest factor and harmonic/inharmonic energy values as evidence even though they are descriptive rather than admission thresholds.

- [ ] **Step 3: Run red**

Run: `cargo test six_op_pm::tests --all-features`

Expected: compilation fails because evidence types/evaluator are absent.

- [ ] **Step 4: Implement exact measurements**

Define `RenderMetrics`, `PitchRow`, `SpectralRow`, `AliasRow`, and `PatchEvidence`. Measure peak, active RMS, DC, crest factor, maximum jump, clamp contacts, finite status, FNV-1a sample hash, harmonic energy, and inharmonic residual energy. Use dedicated sustained-note probes rather than projecting the musical score. Cover MIDI 36/60/84, rational and inharmonic ratios, low/medium/high modulation levels, attack/sustain/release envelope stages, and zero/nominal/maximum declared feedback. For alias rows, render 32,768 post-warmup samples at 48 kHz and 8x, box-decimate, and call `fitted_residual_db`. Floors are `-20 dB` at MIDI 36/60 and `-12 dB` at MIDI 84. Harmonic patches require pitch error within 20 cents; bell/metal patches may instead retain the intended anchor within 36 dB of the strongest partial.

Do not normalize. Adjust only authored parameters or one fixed pair gain when a test fails, and add a focused regression before changing DSP topology.

- [ ] **Step 5: Run green and commit**

Run: `cargo test six_op_pm --all-features && cargo test research::tests --all-features`

Expected: all new acceptance and existing research tests pass.

```bash
git add src/research.rs src/six_op_pm.rs
git commit -m "feat: measure six-operator PM listening gate"
```

## Task 6: CLI and deterministic files

**Files:**
- Create: `src/bin/six-op-pm-lab.rs`
- Create: `tests/six_op_pm_cli.rs`

- [ ] **Step 1: Write failing CLI test**

Run the binary twice into `tempfile` directories. Require byte-identical files except `workstation-cost.txt`, the exact six sorted WAV names, stereo 48 kHz 32-bit float format, and these reports: `README.md`, `manifest.tsv`, `metrics.tsv`, `pitch.tsv`, `spectral.tsv`, `alias-error.tsv`, `graphs.tsv`, `hashes.tsv`.

- [ ] **Step 2: Run red**

Run: `cargo test --test six_op_pm_cli --all-features`

Expected: Cargo cannot find the binary.

- [ ] **Step 3: Implement CLI and report contracts**

Accept only `render <output-directory>`. Refuse an existing non-empty destination. Render and verify all six patches before creating output. Write float WAVs and deterministic reports with these headers:

```text
manifest.tsv: number,patch,pair,role,algorithm,score,filename,pair_gain,status
metrics.tsv: patch,peak,active_rms,active_rms_dbfs,crest_factor,dc,maximum_jump,ceiling_contacts,finite,status
pitch.tsv: patch,note,frequency_hz,pitch_anchor_db,pitch_error_cents,inharmonic_rule,status
spectral.tsv: patch,note,stage,harmonic_energy_db,inharmonic_residual_db
alias-error.tsv: patch,note,alias_error_db,floor_db,status
graphs.tsv: graph,source_number,edges,carriers,feedback_edge,evaluation_order
hashes.tsv: file,fnv1a_interleaved_stereo_sample_hash
```

The README states safe initial playback, pair order, human-listening authority, three topologies rather than six families, no copied Yamaha patch, no per-file normalization, no effects, no compatibility claim, no acoustic-SPL claim, and no production integration. Put volatile scalar timing only in `workstation-cost.txt`.

- [ ] **Step 4: Run green and commit**

Run: `cargo test --test six_op_pm_cli --all-features`

Expected: deterministic CLI test passes.

```bash
git add src/bin/six-op-pm-lab.rs tests/six_op_pm_cli.rs
git commit -m "feat: render six-operator PM listening gate"
```

## Task 7: Release render and bounded tuning

**Files:**
- Modify only after a failing regression: `src/six_op_pm.rs`
- Generate disposable: `artifacts/six-operator-pm-listening-gate/`

- [ ] **Step 1: Render release gate**

Run: `cargo run --release --bin six-op-pm-lab -- render artifacts/six-operator-pm-listening-gate`

Expected: six WAVs, eight deterministic reports, timing file, exit 0.

- [ ] **Step 2: Inspect gates**

Run: `rg -n "fail|reject|false" artifacts/six-operator-pm-listening-gate/{manifest,metrics,pitch,alias-error}.tsv`

Expected: no output. For a failure, keep the diagnostic, add/tighten a regression, adjust only ratios/index/feedback/envelopes/pair gain, and regenerate the entire batch.

- [ ] **Step 3: Verify inventory and determinism**

List files with `find ... -maxdepth 1 -type f -printf '%f\n' | sort`. Render twice into fresh `mktemp -d` directories and compare everything except `workstation-cost.txt` using `diff -qr`; expected: no differences. If any alias row fails, retain the failing measurement as negative evidence and first bound ratio, modulation level, and feedback. Add fixed oversampling only after a matched scalar-cost measurement demonstrates that it clears the row.

- [ ] **Step 4: Commit code tuning only**

```bash
git add src/six_op_pm.rs
git commit -m "fix: keep six-operator PM gate within bounds"
```

Never add artifacts, WAVs, reports, or parameter dumps.

## Task 8: Documentation and knowledge

**Files:**
- Modify: `docs/RESEARCH.md`
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/HANDOFF.md`
- Modify: `/home/shome/Documents/knowledge/Moj Sint/01 Current State.md`
- Modify: `/home/shome/Documents/knowledge/Moj Sint/04 Next Actions And Open Questions.md`

- [ ] **Step 1: Record durable research evidence**

Record official source URLs and licensing boundary, independently authored implementation, exact three graphs/six patches/scores/fixed gains, metrics and alias rows, determinism aggregate, scalar timing boundary, limitations, and open human verdict. Do not copy Yamaha prose, diagrams, patches, or voice data.

- [ ] **Step 2: Update architecture and handoff**

Document the two new modules, delayed feedback semantics, allocation boundary, dual-mono presentation, verification results, and production isolation. Replace “implementation has not started” with observed state and make listening the next gate.

- [ ] **Step 3: Update and validate concise knowledge**

Keep only durable routing, clean-room boundary, implemented/open-verdict state, and next decision. Do not store HEAD, artifact existence, timing, or Git status.

Run:

```bash
git diff --check
/home/shome/Documents/knowledge/.zk/validate.sh
/home/shome/Documents/knowledge/.zk/tests/validate-test.sh
```

Expected: diff check exits 0; both validators report PASS.

- [ ] **Step 4: Commit docs**

```bash
git add docs/RESEARCH.md docs/ARCHITECTURE.md docs/HANDOFF.md
git commit -m "docs: record six-operator PM listening gate"
```

## Task 9: Full verification and human handoff

**Files:**
- Verify all changed source, tests, and docs
- Do not modify production or external systems

- [ ] **Step 1: Run full suite**

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

Expected: every command exits 0; `cargo deny` may retain only the documented accepted duplicate `winnow` warning.

- [ ] **Step 2: Re-run release determinism and decode checks**

Render twice into fresh temporary directories; exclude only timing; require `diff -qr` silence. Decode all six WAVs and require their interleaved FNV-1a hashes to match `hashes.tsv`.

- [ ] **Step 3: Audit real-time and scope boundaries**

Run `rg -n "sin\(|cos\(|powf\(|println!|eprintln!|File::|fs::|Mutex|RwLock" src/dsp/six_op_pm` and inspect each match. Expected: transcendental setup only in preparation and no I/O/logging/locks/files/process/clocks in DSP. Confirm `git status --short` contains no artifacts or unrelated changes.

- [ ] **Step 4: Present the gate and stop**

Give the exact absolute artifact path plus raw `file:///` URI, pair order, low initial playback guidance, evidence boundaries, and open verdict. Do not select/integrate a graph, alter controls, touch JACK/ALSA/SHR-DAW, preserve outputs, push, or claim DX7 compatibility before the user's listening verdict.
