# Isolated Open303 candidate

Historical candidate record: the subsequent [live integration](OPEN303_INTEGRATION.md)
enables Open303 by default and adds a production model and four presets. The
candidate's earlier default-off and no-integration statements below describe
its original validation stage.


Implemented 2026-09-09 after the [source analysis](OPEN303_ANALYSIS.md).
This reuses Robin Schmidt's C++ oscillator/filter/envelope engine through a
small C ABI instead of translating the DSP. It is a default-off offline
candidate, separate from Moj Sint's production Engine and seven-model schema.
The reviewed source, exact pin, manifests, licenses, and repair ledger are in
[vendor provenance](../vendor/open303/README.md). No JC-303 plugin code is used.

## Build and audition

The `open303` feature needs a C++17 compiler and standard library in addition
to the existing Rust/system prerequisites. It has been exercised on this
Linux AArch64 workstation. Cross-compilation needs a matching target C++
toolchain; other platforms are not validated. Default builds do not compile
or link Open303 or require its optional `cc` build dependency.

```sh
cargo run --release --features open303 --bin open303-lab -- artifacts/open303-candidate
```

Use a new folder; existing folders are refused. The command writes two
six-second, 48 kHz float WAVs and a report. Both use the same project-authored
16-step phrase, controls, accent/slide events, gain, and two-second release.
`01-coupled-303.wav` uses the TB_303 coupling; `02-cascade-18.wav` uses LP_18,
the original VST default. These are different filter structures, not a knob
sweep. Files are dual mono, with no normalization or automatic playback.
The report records peak, RMS, ceiling hits, and end-idle state. Generated
batches remain disposable under ignored `artifacts/`; never add them to Git.

## Boundary and behavior

`src/open303.rs` owns one opaque C++ instance. Construction prepares tables,
rate, and mode; Drop destroys it. Neither belongs in an audio callback.
Instances can move between threads but cannot be concurrently shared. Each
instance has independent preparation scratch. Mode/rate are fixed for its
lifetime; changing either requires preparing a replacement outside rendering.

| Control | Accepted range | Default |
| --- | --- | --- |
| Waveform, saw–square blend | 0–1 | 0.5 |
| Cutoff | 314–2394 Hz | 700 Hz |
| Resonance / envelope modulation / accent | 0–100 each | 40 / 35 / 60 |
| Normal decay | 200–2000 ms | 500 ms |
| Volume | −60–0 dB | −24 dB |
| Tuning | 400–480 Hz | 440 Hz |
| Slide | 0–300 ms | 60 ms |

Rates are 44100–96000 Hz inclusive. Both Rust and C reject invalid/nonfinite
controls before any update. Values are cached to skip unchanged setters.
The defaults are project-authored; the lab changes waveform to .65, cutoff
to 900, resonance to 75, envelope modulation to 70, and accent to 60.

MIDI keys/velocities are 0–127; velocity zero is note-off. Up to 128 unique
held keys use bounded fixed storage. Repeated note-ons replace that key's
entry. The most recent held note wins; returning to an earlier key restores
its tuned pitch and accent. Overlap invokes inherited slide behavior; stale
note-offs do nothing. Release-all permits the envelope tail, while reset
clears note/audio/envelope/fault state to silence and retains controls/tables.
The patched core returns to idle after 64 quiet samples below 1e-9 once the
amplitude gate is off and no keys remain.

Render, note events, reset, and control calls have no dynamic allocation in
the tested boundary. There is no I/O, synchronization, or host access in the
DSP adapter. Controls apply when called; tuning, accent amount, and normal
decay can affect held notes. Controls are not smoothed in this baseline.
Live integration still needs sample-offset event scheduling, smoothing,
coefficient-update budgeting, denormal/cost assessment, and native callback
measurements. Finite/allocation tests alone do not establish those properties.

The adapter clamps finite output to ±0.999 and counts every clipped sample.
This is a presentation safeguard, not evidence of native gain/headroom or
analog saturation. A nonfinite sample resets internal state and latches silent
output until explicit reset; status exposes that fault. There is no claim of
hardware fidelity. The active inherited TB_303 path remains linear in audio
amplitude. Listening and comparison against a chosen reference remain open.

## Validation and reproduction

Normal optional-feature tests cover configuration rejection without mutation,
all 128 keys, note restoration, live controls, reset/idle, concurrent
preparation, deterministic chunking, distinct modes, finite bounded output,
Rust and C++ allocation/deallocation guards, direct C/core sample agreement,
fault-to-silence and recovery, FFT roundtrip, and CLI repeatability/refusal to
overwrite. The native test uses white-box fault injection, without exposing
an injection API in the library. Sanitizers exercise the actual imported code.

```sh
cargo test --locked --all-targets --all-features
tools/check-open303-native.sh
# One-time oscillator evidence, intentionally opt-in:
cargo test --features open303 --test open303_contract -- --ignored --nocapture
# Or combine native sanitizers and that evidence:
tools/check-open303-native.sh --spectral
(cd vendor/open303 && sha256sum -c LOCAL.sha256)
```

The spectral check isolates oscillator plus four-times decimation at 48 kHz,
using a coherent 65536-sample FFT after equal warmup. It compares all other
non-DC bin power against intended harmonic power; it is a residual metric,
not a universal alias floor for modulated/full-voice material.

| Fundamental | Saw residual | Square residual |
| --- | ---: | ---: |
| 40.283203 Hz | −64.596 dB | −69.291 dB |
| 330.322266 Hz | −90.983 dB | −95.507 dB |
| 2499.755859 Hz | −115.561 dB | −117.467 dB |

These match the earlier source-analysis probe to reported precision. The
normal suite keeps focused regressions; the six spectral measurements and
unrelated historical auditions/benchmarks stay opt-in. No JACK, SHR, hardware,
listening, or Pi performance acceptance is implied by these software checks.

Release checks validated all 24 production presets and found byte-identical
paired one-second renders at MIDI 40, velocity .85, 48 kHz. The candidate also
repeats byte-for-byte within each debug/release profile. Across profiles its
largest observed sample difference was 3.7253e-9; cross-profile bit identity is
not promised. The authored audition peaks were .111985482 (TB_303) and
.103063203 (LP_18), with zero ceiling hits and both voices idle at the end.
These measurements describe this phrase only, not the full control range.

Final normal all-target/all-feature suite: 356 passed, 36 intentionally ignored.
Focused candidate tests also passed in release; formatting, warning-denied
Clippy, release build, audit/deny, and manifest checks passed. A default-feature
library check succeeded with an unusable CXX path, confirming that the optional
native build is not required for the default library.
