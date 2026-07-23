# Final Hybrid Sound Lab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build three complete, structurally distinct, genuinely stereo hybrid voice experiments and a deterministic single-note, three-note chord, progression, and mono-fold listening flow.

**Architecture:** A new isolated `dsp::hybrid` module owns fixed-state scalar primitives and three complete voice topologies. A non-real-time `hybrid` module schedules one- and three-note auditions, measures stereo/harmonic/alias evidence, and a separate `hybrid-sound-lab` binary writes the disposable twelve-file batch and reports. Nothing routes into production `Engine`, presets, macros, JACK/ALSA, or SHR-DAW.

**Tech Stack:** Stable scalar Rust 1.97.1, fixed-size arrays, `assert_no_alloc`, existing `hound` WAV support, deterministic offline analysis, Cargo audit/deny, and AArch64 compile checks.

**Research basis:** Follow `docs/BASS_IMPACT_RESEARCH.md`: separate the short
onset cue from the slower low-band body, preserve a centered fundamental and
sparse per-note harmonic spine, keep nonlinear processing per voice before the
linear chord mix, reserve stereo mid energy, and treat crest, roughness,
correlation, and side/mid as descriptive ranges rather than quality scores.
Do not claim acoustic pressure, tactile response, or standardized true peak
from normalized workstation WAVs.

**Predeclared engineering rejection bounds:** Reject any non-finite sample,
sample-path allocation, pre-listening sample peak above `1.0`, sustained output
DC magnitude above `5e-4`, sample-identical true-stereo channels, side/mid
energy below `0.005`, mono-fold RMS below half the stereo RMS, a target chord
note more than `36 dB` below the strongest target-note projection, feedback
energy that grows after excitation ends, or a declared slow spatial movement
that does not complete two cycles inside its listening render. Report rather
than hide high-rate residuals, crest factor, correlation, roughness-related
modulation, and low-band side/mid ranges.

---

### Task 1: Hybrid API and fixed-state primitives

**Files:**
- Create: `src/dsp/hybrid/mod.rs`
- Create: `src/dsp/hybrid/primitives.rs`
- Modify: `src/dsp/mod.rs`

- [x] **Step 1: Write failing API and primitive tests**

Add tests requiring:

```rust
let mut voice = HybridVoice::new(
    HybridFamily::CrossCoupledMachine,
    48_000.0,
    220.0,
    0x1234_5678,
)?;
let first = voice.sample();
voice.reset();
assert_eq!(first, voice.sample());
```

Test invalid sample rates/frequencies, deterministic prepared quadrature
rotation, explicit 8/12/16-bit masked wrapping and carry events, a finite
two-stage impact envelope with a 2–5 ms upper cue and 10–40 ms body rise,
per-note sparse harmonic-spine phase preparation, bounded one-pole split
reconstruction, stable all-pass state, fixed-delay bounds, moving-delay
continuity, resonator feedback below unity, finite nonlinear helpers, and no
allocation inside every primitive sample method.

- [x] **Step 2: Run the focused tests and verify RED**

Run:

```sh
cargo test dsp::hybrid::tests -- --nocapture
```

Expected: compilation fails because `dsp::hybrid` and `HybridVoice` do not
exist.

- [x] **Step 3: Implement the minimal shared boundary**

Define:

```rust
pub enum HybridFamily {
    CrossCoupledMachine,
    SpectralShadow,
    DualResonantBody,
}

pub struct HybridFrame {
    pub left: f32,
    pub right: f32,
}

pub struct HybridVoice {
    state: HybridState,
}
```

Implement fixed-size prepared primitives for recursive oscillation, triangle
movement, one small register oscillator, a two-stage impact envelope, a sparse
same-note 2nd/3rd/5th harmonic spine, one-pole splitting, first-order all-pass
phase shift, linear-interpolated delay, damped resonator, DC blocking, bounded
odd and asymmetric soft drive, generated-component extraction, and a soft
high-band gate. All coefficient preparation and `sin_cos` calls occur in
constructors.

- [x] **Step 4: Run focused tests and verify GREEN**

Run:

```sh
cargo test dsp::hybrid::tests -- --nocapture
```

Expected: primitive/API tests pass with no allocation.

- [x] **Step 5: Commit**

```sh
git add src/dsp/hybrid src/dsp/mod.rs
git commit -m "feat: add fixed-state hybrid DSP boundary"
```

### Task 2: Cross-coupled machine voice

**Files:**
- Create: `src/dsp/hybrid/cross_coupled.rs`
- Modify: `src/dsp/hybrid/mod.rs`

- [x] **Step 1: Write failing topology tests**

Require deterministic reset; finite samples bounded by `1.0`; retained base
pitch; upper onset preceding the body rise; a centered fundamental plus sparse
translation harmonics; nonzero register contribution; reconstructed low/high
split; per-voice rather than post-mix nonlinearity; audible subharmonic without
dominant DC; different low-pulse, vibrato, and stereo movement rates; two safe
cross-coupled resonator pairs; distinct L/R samples; the predeclared stereo and
mono bounds; maximum adjacent jump below the declared bound; and
allocation-free sampling.

- [x] **Step 2: Run and verify RED**

```sh
cargo test dsp::hybrid::tests::cross_coupled -- --nocapture
```

Expected: failure because the family is not constructed.

- [x] **Step 3: Implement the complete voice**

Implement the approved flow:

```text
impact envelope + prepared vibrato -> nonlinear PM carrier + harmonic spine
register state/carry -> PM perturbation + phase-shifted subharmonic
carrier split -> low bounded odd overdrive with independent pulse movement
              -> high asymmetric generated-component gate/distortion
low + high + sub -> asymmetric L/R resonator pairs with safe cross-feedback
                 -> independently moving micro-delays -> bounded stereo frame
```

Use one register mechanism, not an array/swarm. Keep a centered component for
mono survival but set gains so it cannot make L/R effectively identical.

- [x] **Step 4: Run focused and hybrid tests**

```sh
cargo test dsp::hybrid::tests::cross_coupled -- --nocapture
cargo test dsp::hybrid::tests -- --nocapture
```

Expected: all pass.

- [x] **Step 5: Commit**

```sh
git add src/dsp/hybrid
git commit -m "feat: add cross-coupled machine voice"
```

### Task 3: Spectral shadow voice

**Files:**
- Create: `src/dsp/hybrid/spectral_shadow.rs`
- Modify: `src/dsp/hybrid/mod.rs`

- [x] **Step 1: Write failing topology tests**

Require four authored spectral states, exact prepared partial rotations,
high-note partial guarding, smoothed fixed-width address perturbation, a
phase-shifted lower shadow, two different nonlinear filter states with bounded
cross-coupling, independent traversal/spatial rates, spectral change across
early/middle/late windows, separate `f0`, optional `f0/2`, and upper
virtual-bass-spine evidence, retained fundamental, true stereo/mono survival,
and allocation-free bounded sampling. In chord mode the real `f0/2` shadow
must use the reduced headroom-safe level declared by the implementation.

- [x] **Step 2: Run and verify RED**

```sh
cargo test dsp::hybrid::tests::spectral_shadow -- --nocapture
```

- [x] **Step 3: Implement the complete voice**

Implement:

```text
authored partial frames <- smoothed address-machine perturbation
                |
          spectral main
                +-> divided/register shadow -> all-pass phase shift
main + shadow -> asymmetric nonlinear L/R filter pair
              <-> bounded cross-state coupling
               -> independent slow stereo motion -> bounded stereo frame
```

Normalize authored frames during construction and taper partials before
Nyquist. Do not copy commercial wavetables or presets.

- [x] **Step 4: Run focused and hybrid tests**

```sh
cargo test dsp::hybrid::tests::spectral_shadow -- --nocapture
cargo test dsp::hybrid::tests -- --nocapture
```

- [x] **Step 5: Commit**

```sh
git add src/dsp/hybrid
git commit -m "feat: add spectral shadow voice"
```

### Task 4: Dual resonant body

**Files:**
- Create: `src/dsp/hybrid/dual_body.rs`
- Modify: `src/dsp/hybrid/mod.rs`

- [x] **Step 1: Write failing topology tests**

Require deterministic onset noise; a quiet pitch-bearing nonlinear exciter;
sparse explicit carry/borrow events; two unequal resonant bodies; low-body
overdrive and high-body soft gating inside distinct feedback paths; bounded
cross-feedback; independent body-delay rates; decaying but sustained energy;
pitch retention; per-mode note tracking; internal-state DC bounds; old-chord
energy below the transition limit before the following segment settles; no
runaway/lockup; true stereo/mono survival; and allocation-free bounded
sampling.

- [x] **Step 2: Run and verify RED**

```sh
cargo test dsp::hybrid::tests::dual_body -- --nocapture
```

- [x] **Step 3: Implement the complete voice**

Implement:

```text
quiet nonlinear carrier + deterministic onset burst + register edge events
                |
       +--------+---------+
       v                  v
low resonant body <-> dispersed high resonant body
feedback overdrive     feedback soft gate
       |                  |
moving body delay L   independently moving body delay R
       +------ safe stereo/mono-compatible mix ------+
```

Keep each feedback coefficient below its tested energy limit and DC-block the
final channels.

- [x] **Step 4: Run focused and hybrid tests**

```sh
cargo test dsp::hybrid::tests::dual_body -- --nocapture
cargo test dsp::hybrid::tests -- --nocapture
```

- [x] **Step 5: Commit**

```sh
git add src/dsp/hybrid
git commit -m "feat: add dual resonant body voice"
```

### Task 5: Three-note ensemble and harmonic scheduler

**Files:**
- Create: `src/hybrid.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Write failing ensemble tests**

Define test-facing types:

```rust
pub enum HybridCondition {
    Single,
    HeldChord,
    Progression,
    ProgressionMono,
}

pub struct HybridRenderSpec {
    pub sample_rate: u32,
    pub family: HybridFamily,
    pub condition: HybridCondition,
}
```

Test one voice for `Single`, exactly three independently seeded voices for
`HeldChord`, the fixed Dm/B-flat/C/Am triples for `Progression`, deterministic
four-second transitions, conservative mix headroom, non-identical phase/state,
finite bounded output, per-voice nonlinear processing followed by a linear
mix, non-silent mono fold, two deterministic alternative seed sets in
engineering tests, and allocation-free three-voice ensemble sampling.

- [x] **Step 2: Run and verify RED**

```sh
cargo test hybrid::tests::ensemble -- --nocapture
```

- [x] **Step 3: Implement scheduling and rendering**

Implement fixed `HybridEnsemble` storage for three voices and offline rendering
for:

```rust
const PROGRESSION: [[u8; 3]; 4] = [
    [50, 53, 57],
    [46, 50, 53],
    [48, 52, 55],
    [45, 48, 52],
];
```

Use ten-second single, twelve-second held chord, and sixteen-second progression
durations. Apply short deterministic chord-transition ramps and offline fades.
For `ProgressionMono`, write the same mid signal to both output channels only
after measuring the original stereo progression.

- [x] **Step 4: Run focused tests**

```sh
cargo test hybrid::tests::ensemble -- --nocapture
```

- [x] **Step 5: Commit**

```sh
git add src/hybrid.rs src/lib.rs
git commit -m "feat: add hybrid chord audition flow"
```

### Task 6: Hybrid measurements and alias evidence

**Files:**
- Modify: `src/hybrid.rs`

- [x] **Step 1: Write failing measurement tests**

Require deterministic metrics for sample peak, four-times interpolated peak
(explicitly not ITU-R true peak), active RMS, DC, crest factor, maximum jump,
time to upper-cue peak, time to body energy rise, decay/residual energy,
full-band and low-band correlation/mid/side, mono RMS and spectral loss, L/R
difference RMS, hash, finiteness, target-note projections, and note-pair/chord
non-harmonic products. Require all three chord notes within the predeclared
retention window for held chords and each progression segment. Compare
per-voice processing with a diagnostic shared-post-mix nonlinearity to expose
cross-note IMD. Require eight-times conservative residual evidence at MIDI
36/60/84 for every family, with severe integer/sample-rate residual retained
rather than converted into a passing score.

- [x] **Step 2: Run and verify RED**

```sh
cargo test hybrid::tests::measure -- --nocapture
```

- [x] **Step 3: Implement measurement helpers**

Add `HybridMetrics`, onset/body timing, simple documented band-energy
measurement, chord-segment note projections, note-pair/chord residual tables,
long-window stereo/low-band/mono measurements, conservative high-rate
render/box-decimation/gain-fit residual, topology comparison hashes, and scalar
timing hooks. Offline code may allocate; `HybridVoice::sample` and
`HybridEnsemble::sample` may not.

- [x] **Step 4: Run measurement and all hybrid tests**

```sh
cargo test hybrid::tests -- --nocapture
cargo test dsp::hybrid::tests -- --nocapture
```

- [x] **Step 5: Commit**

```sh
git add src/hybrid.rs
git commit -m "feat: measure hybrid stereo and harmony"
```

### Task 7: Disposable twelve-file lab

**Files:**
- Create: `src/bin/hybrid-sound-lab.rs`
- Create: `tests/hybrid_sound_cli.rs`

- [x] **Step 1: Write the failing CLI integration test**

Invoke:

```sh
hybrid-sound-lab render <temporary-directory>
```

Require exactly these twelve 48 kHz, stereo, 32-bit-float WAVs:

```text
cross-coupled-machine_{single,chord,progression,progression-mono}.wav
spectral-shadow_{single,chord,progression,progression-mono}.wav
dual-resonant-body_{single,chord,progression,progression-mono}.wav
```

Require `manifest.tsv`, `impact.tsv`, `stereo.tsv`, `harmony.tsv`, `products.tsv`,
`alias-error.tsv`, `topology.tsv`, `workstation-cost.txt`, and `README.md`.
Generate twice into fresh directories and require byte-identical WAVs/reports
except volatile cost. The README must distinguish sample peak from the
non-standard interpolated peak, true stereo from the mono-fold diagnostics,
waveform design from playback SPL, and state that no sound is accepted.

- [x] **Step 2: Run and verify RED**

```sh
cargo test --test hybrid_sound_cli -- --nocapture
```

Expected: failure because the binary does not exist.

- [x] **Step 3: Implement the lab CLI**

Render all conditions, loudness-match comparable musical conditions under a
conservative peak ceiling, write WAVs and reports, and provide the documented
headphone/speaker/mono listening order. Keep all output under the caller's
explicit directory.

- [x] **Step 4: Verify deterministic CLI output**

```sh
cargo test --test hybrid_sound_cli -- --nocapture
```

- [x] **Step 5: Commit**

```sh
git add src/bin/hybrid-sound-lab.rs tests/hybrid_sound_cli.rs
git commit -m "feat: render final hybrid sound lab"
```

### Task 8: Release evidence, listening batch, and durable documentation

**Files:**
- Modify: `docs/ARCHITECTURE.md`
- Modify: `docs/RESEARCH.md`
- Modify: `docs/HANDOFF.md`
- Modify: `docs/superpowers/plans/2026-07-23-final-hybrid-sound-lab.md`
- Generate only under: `artifacts/final-hybrid-sound-lab/`

- [x] **Step 1: Generate two fresh release batches**

```sh
cargo run --release --bin hybrid-sound-lab -- render /tmp/moj-sint-hybrid-a
cargo run --release --bin hybrid-sound-lab -- render /tmp/moj-sint-hybrid-b
```

Compare recursive hashes excluding only `workstation-cost.txt`. Investigate any
difference before proceeding.

- [x] **Step 2: Review release evidence**

Check every report for finite output, bounded peaks, DC, onset/body timing,
crest factor, chord-note retention, non-harmonic products, actual L/R
difference, full/low-band correlation and side/mid, mono survival, movement
coverage, feedback decay, alias residual, deterministic hashes, and distinct
topology evidence. Fix defects test-first; do not tune musical taste from
metrics alone.

- [x] **Step 3: Run the full repository matrix**

```sh
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
cargo audit
cargo deny check
cargo check --target aarch64-unknown-linux-gnu
git diff --check
```

Only the already accepted duplicate `winnow` warning may remain.

- [x] **Step 4: Record measured facts**

Update architecture, research, and handoff with topology, exact measurements,
limitations, deterministic evidence, chord flow, and open human verdict. State
that no family, macro, Pi performance, safe polyphony, headphone/speaker
translation, or final sound is accepted.

- [x] **Step 5: Produce the disposable user batch**

```sh
cargo run --release --bin hybrid-sound-lab -- render artifacts/final-hybrid-sound-lab
```

Give the user the absolute path and listening order. Do not preserve it
elsewhere or delete it before the user's verdict.

- [x] **Step 6: Commit the durable implementation and docs**

```sh
git add src tests docs
git commit -m "docs: record final hybrid listening gate"
```
