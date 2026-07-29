# Model D Character Model Design

**Date:** 2026-07-29  
**Status:** Approved design, awaiting implementation plan  
**Owner:** Moj Sint

## Purpose

Build an isolated, monophonic, circuit-informed character model of the
Minimoog Model D signal path. The experiment will test how effectively Moj
Sint can reproduce the character of a documented existing instrument when the
implementation models the interactions among its oscillators, mixer, filter,
contours, amplifier, and feedback rather than applying a generic effect to one
weak source.

The result is a playable research voice, not a forensic claim of equivalence
to a particular hardware unit. Human listening will decide whether it has a
convincing and useful identity. Automated evidence will establish only the
declared DSP, safety, and reproducibility properties.

## Evidence and intellectual-property boundary

The implementation may use factual signal-flow and circuit information and
independently implement published equations. It must not copy third-party DSP
source, factory presets, samples, recordings, manual prose, figures, or circuit
artwork.

Primary technical sources:

- Moog Music, *Minimoog Model D Manual*, official reissue manual:
  <https://back.moogmusic.com/sites/default/files/2022-11/Minimoog_Model_D_Manual.pdf>.
  It documents three oscillators, mixer overload, the four-pole ladder filter,
  dual contours, and output-to-external-input feedback.
- Robert A. Moog, US Patent 3,475,623, *Electronic high-pass and low-pass
  filters employing the base to emitter diode resistance of bipolar
  transistors*: <https://patents.google.com/patent/US3475623A/en>.
  It establishes the transistor-ladder topology and its voltage-controlled
  resistance principle.
- Antti Huovilainen, “Non-Linear Digital Implementation of the Moog Ladder
  Filter,” DAFx-04:
  <https://dafx.de/paper-archive/2004/P_061.PDF>.
  It derives a practical nonlinear cascade from the circuit equations and
  documents tuning correction and oversampling concerns.

The research CLI and documentation may use “Model D” descriptively to identify
the studied architecture. Moj Sint must not imply affiliation, endorsement, or
bit-exact reproduction.

## Scope

### Included

- Three independently phased, bandlimited VCOs.
- Per-oscillator octave offsets and selectable triangle, saw, rectangle, wide
  pulse, and narrow pulse families needed by the audition patches.
- Small, bounded, deterministic differences among oscillators:
  static tracking offset, slow pitch drift, waveform asymmetry, pulse-width
  offset, and level offset.
- A level-controlled summing mixer with a linear diagnostic mode and a bounded
  nonlinear mode whose harmonic generation and compression increase with
  summed source level.
- A four-stage nonlinear low-pass ladder derived independently from the
  documented equations, with cutoff correction, resonance feedback, fixed
  oversampling, and a linear-stage diagnostic mode.
- Separate filter and loudness contours using attack, decay, and sustain, with
  decay time reused for release.
- A bounded output-to-input feedback path.
- A monophonic prepared voice with an allocation-free sample path.
- An offline lab that renders a small musical audition and matched causal
  ablations under ignored `artifacts/`.
- Deterministic engineering reports for oscillator, mixer, filter, render,
  aliasing, and workstation-cost evidence.

### Excluded

- Production `Engine`, preset-schema, stable macro, JACK, ALSA, SHR-DAW, or
  controller integration.
- Polyphony, keyboard-priority emulation, legato trigger modes, pitch-wheel
  behavior, noise source, external audio input, and a general modulation
  matrix.
- Component-tolerance Monte Carlo simulation, electrical SPICE accuracy, or
  calibration to an individual hardware serial number.
- Copied Model D factory patches, recorded comparison material, or claims that
  the experiment is indistinguishable from hardware.
- Raspberry Pi performance, latency, or polyphony claims without native
  evidence.

## Architecture

The implementation remains isolated from `Engine`:

```text
note and patch
      |
      v
pitch preparation
      |
      +---------> VCO 1 ----+
      +---------> VCO 2 ----+--> nonlinear mixer --> nonlinear ladder --> VCA
      +---------> VCO 3 ----+          ^                  ^              ^
                                         \                 |              |
                                          output feedback  |       loudness contour
                                                           |
                                                    filter contour
```

The voice is prepared when the sample rate, note, or patch changes. Preparation
computes phase increments, drift rotations, pulse-width bounds, contour rates,
cutoff mapping, ladder coefficients, oversampling state, feedback gains, and
output gain. The per-sample path performs only fixed-size scalar arithmetic,
table lookup/interpolation, bounded rational nonlinearities, and state updates.
It does not allocate, deallocate, lock, perform I/O, log, format, panic, spawn,
or call avoidable per-sample transcendental functions.

### Module boundaries

- `src/model_d/vco.rs` owns one oscillator's bandlimited waveform generation,
  prepared pitch, deterministic drift, asymmetry, and reset state.
- `src/model_d/mixer.rs` owns source levels, summation, nonlinear transfer, and
  its linear diagnostic mode.
- `src/model_d/ladder.rs` owns the four filter stages, oversampling and
  decimation state, resonance loop, cutoff correction, and linear diagnostic
  mode.
- `src/model_d/contour.rs` owns the attack-decay-sustain contour whose decay
  rate is also used after note-off.
- `src/model_d/voice.rs` owns the prepared patch, three VCOs, both contours,
  feedback routing, note lifecycle, and mono sample output.
- `src/model_d.rs` exposes only the types needed by tests and the research lab.
- `src/model_d_lab.rs` owns offline scores, render metrics, alias comparison,
  report rows, and deterministic file naming.
- `src/bin/model-d-lab.rs` parses the output directory and writes disposable
  WAV and report files.

## Component behavior

### VCO bank

Each VCO uses an independent phase accumulator and the repository's established
bandlimited-waveform discipline. The bank will not simulate random analog
behavior. Every imperfection is deterministic for the same patch, note, sample
rate, and reset.

Each oscillator has:

- a musical range offset;
- a static tuning offset expressed in cents;
- a slow, smooth, zero-mean drift path with a distinct deterministic phase;
- waveform-specific asymmetry or pulse-width offset;
- a small level offset; and
- an independent reset phase.

Default full-model bounds:

- static tuning offset: at most 2.5 cents from the requested oscillator pitch;
- slow drift excursion: at most 1.5 cents around that static offset;
- mean fundamental pitch: within 5 cents of the configured musical interval;
- pulse width: clamped between 10% and 90%;
- no discontinuous random pitch steps.

The idealized diagnostic disables static mismatch, drift, asymmetry, and level
offset while retaining the same notes, waveforms, phase starts, score, and
output measurement.

### Mixer

The mixer receives the three VCO outputs after their independent level
controls. Its nonlinear mode uses a bounded odd transfer with no transcendental
call in the sample path. Increasing simultaneous source level must:

- remain monotonic for the declared operating range;
- reduce incremental gain before hard digital clipping;
- introduce measurable harmonics;
- preserve the fundamental and pitch; and
- remain finite for every finite input.

The linear diagnostic uses the same levels and explicit output compensation but
skips the nonlinear transfer. No limiter, compressor, or automatic
per-render normalization may conceal mixer behavior.

### Ladder filter

The ladder contains four cascaded stateful low-pass stages and a resonance path
from the final output to the input differential stage. Every nonlinear stage
uses the same bounded transfer family, with state and coefficients prepared
outside the sample path.

The first implementation uses a fixed four-times internal sample rate and a
fixed-size decimation filter. It remains scalar Rust. Oversampling is part of
the sound path, not an offline-only cleanup step.

At low drive and low resonance, automated probes must establish:

- monotonic movement of measured cutoff as the cutoff control increases;
- an asymptotic low-pass slope consistent with a four-stage cascade, accepted
  between 20 and 28 dB per octave in the declared measurement region;
- cutoff error no greater than 8% at the calibration points after correction;
- increasing resonance produces an increasing peak near cutoff; and
- filter state remains finite and bounded throughout the declared control
  range.

At musical drive, stage nonlinearity must create a measurable difference from
the linear-stage diagnostic without replacing a pitched input with broadband
noise. Resonance is bounded to the stable audition range. Deliberate
self-oscillation and unstable feedback are excluded from this milestone.

### Contours and feedback

The filter and loudness contours share the same state-machine semantics but
hold independent settings. Each has attack, decay, and sustain controls.
Note-off transitions to a release segment that uses the configured decay time.
The output must reach exact idle zero after the bounded release tail.

The filter contour adds a prepared pitch-scaled offset to cutoff. The loudness
contour controls final amplitude.

The optional feedback path routes a delayed, bounded fraction of voice output
back before the mixer/filter boundary. Its declared maximum must preserve
recognizable pitch and finite decay. The full-model audition uses feedback
below the point at which the official manual warns that overload can obscure
different pitches.

## Audition contract

The lab writes exactly seven WAV files at 48 kHz:

1. `01_full_bass_phrase.wav`
2. `02_full_lead_phrase.wav`
3. `03_full_filter_articulation.wav`
4. `04_matched_idealized_path.wav`
5. `05_matched_linear_mixer.wav`
6. `06_matched_linear_ladder.wav`
7. `07_matched_no_drift_or_feedback.wav`

Files 1–3 demonstrate note, register, envelope, and filter-articulation
coverage inside one modeled instrument. They are not presented as distinct
sound-generation families. Files 4–7 replay the same bass score and settings
as file 1 while disabling only the named mechanism. They are causal diagnostic
comparisons, not musical variations.

The scores and patch values are authored for Moj Sint. They do not reproduce
factory presets or copyrighted recordings. All files use explicit fixed gains.
The lab must not normalize each file, add reverb/delay, or hide overload with a
full-band limiter.

Generated WAVs and reports remain disposable under
`artifacts/model-d-character/`. They are never added to Git automatically.

## Evidence and acceptance gates

### Unit and render-path gates

- Invalid sample rates and non-finite configuration values are rejected during
  preparation.
- Prepared VCO, mixer, ladder, contour, and complete-voice sample paths are
  deterministic, finite, resettable, and allocation-free.
- VCO pitch and drift stay within the declared bounds across MIDI notes 36, 60,
  and 84 at 44.1, 48, and 96 kHz.
- The nonlinear mixer demonstrates compression and harmonic generation relative
  to its matched linear mode.
- The ladder passes the declared slope, cutoff, resonance, finiteness, and
  bounded-state checks.
- Contours complete their attack/decay/release transitions sample-accurately
  and return to exact idle zero.
- Voice output has absolute DC no greater than 0.005, no non-finite samples, no
  sample jump greater than 0.25, no digital clipping, and at least 1 dBFS sample
  headroom in every final audition.
- The matched ablations differ from the full bass render while keeping the
  score, duration, and explicit presentation gain identical.

### Aliasing gate

Oscillator and nonlinear changes require high-rate evidence. The lab compares
the 48 kHz full path with a time-aligned 192 kHz reference decimated by the
same declared analysis filter. Across representative low, middle, and high
notes:

- low- and middle-note residual must be at or below -45 dB relative to the
  reference signal;
- the high-note residual must be at or below -35 dB;
- all reported values must be finite and deterministic; and
- a failure remains visible in the report instead of being omitted or
  reclassified.

These residuals are engineering rejection bounds, not proof that the model
sounds analog or matches hardware.

### Reproducibility and repository gates

Two fresh release generations must match byte-for-byte except for an explicitly
excluded workstation timing report. The CLI test verifies the exact file set,
report schemas, fixed gains, absence of hidden normalization/limiting, passing
rows, and deterministic hashes.

Before handoff, run:

- `cargo fmt --check`
- `cargo test --all-targets --all-features`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release`
- `cargo audit`
- `cargo deny check`
- `cargo check --target aarch64-unknown-linux-gnu --all-targets --all-features`
- a deterministic two-generation render comparison
- `git diff --check`
- `/home/shome/Documents/knowledge/.zk/validate.sh`

AArch64 compilation is portability evidence only, not Raspberry Pi performance
evidence.

## Human listening gate

Automated checks cannot accept the model musically. Listening should answer:

- Does the full path sound like one coherent instrument rather than a saw bank
  followed by distortion?
- Do the three oscillators create useful weight and movement without sounding
  detuned or chorus-like?
- Does increased mixer level change density and articulation without turning
  the sound into noise?
- Does the ladder retain body as cutoff and resonance move?
- Are the differences in the matched ablations audible but proportionate?
- Are bass, lead, and articulation examples playable and recognizable at
  matched listening level?

Until that verdict is positive, the implementation remains an isolated
research model. It is not routed into `Engine`, presets, stable macros, JACK,
ALSA, or SHR-DAW.

## Durable handoff

After implementation and fresh verification:

1. update `docs/HANDOFF.md` with the exact implemented scope, engineering
   evidence, limitations, artifact path, and pending listening questions;
2. update `docs/RESEARCH.md` with source provenance and the distinction between
   causal fidelity and hardware equivalence;
3. update the concise Moj Sint knowledge note without volatile Git or artifact
   snapshots; and
4. validate the knowledge notebook.

No push is authorized by this task.
