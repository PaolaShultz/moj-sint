# Moj Sint workspace handoff

Last updated: 2026-07-22, Europe/Zagreb.

This is the durable starting point for a fresh Codex session in
`/home/shome/p/moj-sint`. Read this file before planning or changing the
workspace. Update it at meaningful checkpoints so later sessions do not have
to reconstruct decisions from chat history.

## Goal

Build a low-level, headless Rust synthesizer focused on sophisticated,
deliberately modified oscillators and genuinely unfamiliar sounds. It will be
one of several software instruments managed by SHR-DAW, not a standalone DAW
and not a replacement or compatibility disguise for synthv1.

The primary hardware target is a Raspberry Pi 5 with 2 GB RAM, 64-bit
Raspberry Pi OS, and NVMe. Sources must also build and run on the current
x86_64 Ubuntu development machine. Do not make performance, latency,
polyphony, or sound-quality claims before measuring them on the actual Pi.

There is no UI in this repository. Control comes through MIDI and SHR-DAW.

## User working preferences and authorization

- The user explicitly authorizes installing any tools and user-local
  dependencies needed for this work. Do not repeatedly ask for permission for
  ordinary in-scope installation.
- The assistant cannot use sudo. Report any required system package with the
  exact command for the user to run.
- If a tool would materially accelerate or improve the work, mention and
  recommend it instead of silently working around its absence.
- Keep the OpenAI documentation MCP disabled unless a real OpenAI-specific
  question requires it. It has recently been slow and is unnecessary for this
  project setup.
- Research should preserve direct links, authorship, publication details,
  licensing implications, and notes about why each source matters.
- New-task prompts should state project work, context, and acceptance criteria;
  they should not copy generic agent or skill instructions that the next
  session will already load.

## Current workspace state

At this checkpoint:

- Git is initialized on clean `main`; the foundation feature branch was merged
  locally and its temporary worktree was removed.
- The Rust crate contains the portable DSP/control/preset/engine library,
  offline renderer/validator binary, tests, reference preset, and documentation.
- The first bandlimited-oscillator milestone is implemented: fair scalar
  PolyBLEP and integrated-wavetable candidates were measured, the integrated
  wavetable was selected by the recorded rule, and smoothed `SHAPE`/`COLOR`
  routes now drive the engine.
- Twenty-seven deterministic listening WAVs cover MIDI notes 36/60/84 and all
  low/mid/high `SHAPE`/`COLOR` combinations under
  `artifacts/oscillator-milestone/`. Human musical-usefulness acceptance is
  intentionally still open.
- User-local stable Rust 1.97.1, Cargo, rustfmt, Clippy, cargo-audit, and
  cargo-deny are installed.
- The live JACK/ALSA host is not implemented and no audio service or hardware
  was touched.
- SHR-DAW was rechecked read-only at main commit
  `8b7d0d7c17c582292ac06a915ca1fe750d77bc40` using a temporary clone.

## Development-machine audit

Current machine:

- Ubuntu 24.04.4 LTS, x86_64
- Linux 6.8.0-136-generic
- AMD Ryzen 5 5600G, 6 cores / 12 threads
- 62 GiB RAM
- approximately 214 GiB free on the workspace filesystem at audit time

Present tools and runtimes:

- GCC/G++ 13.3
- Make, CMake 3.28, `pkg-config` 1.8
- Git 2.43 and authenticated GitHub CLI
- JACK 1.9.21 runtime
- PipeWire 1.0.5 runtime
- ALSA runtime utilities and visible playback devices
- `perf`, `gdb`, `chrt`, and `taskset`
- Rust 1.97.1, Cargo 1.97.1, rustfmt, Clippy, cargo-audit, and cargo-deny
- ALSA development package `libasound2-dev` 1.2.11-1ubuntu0.3; `pkg-config`
  resolves `alsa` 1.2.11

Still missing:

- JACK development metadata
- PipeWire development metadata
- Clang/LLD, Valgrind, rr, hyperfine, and just

Rust and ALSA development metadata were missing during the original machine
audit and are now installed as recorded in the present-tools list. The
remaining tools are not needed for the current portable foundation and should
not be installed speculatively.

Do not install every missing item automatically. Install only what the chosen
architecture or validation actually needs. On this development machine,
`libasound2-dev` is already installed. A fresh Raspberry Pi OS machine will
likely need:

```sh
sudo apt update
sudo apt install libasound2-dev
```

Whether JACK development headers are required depends on the selected Rust
boundary. SHR-DAW itself dynamically loads `libjack.so.0`, avoiding a build-time
JACK development dependency. Evaluate that approach against a maintained Rust
JACK crate before choosing.

## SHR-DAW integration source of truth

The repository named in the first discussion had a spelling mismatch. The
actual public repository is:

<https://github.com/PaolaShultz/shr-daw>

The GitHub owner is `PaolaShultz`, with a final `z`.

The inspection used the current `main` branch on 2026-07-22. Recheck current
files in a new session because this repository is actively changing. Important
sources are:

- [`src/engine.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/engine.rs)
- [`src/preset.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/preset.rs)
- [`src/config.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/config.rs)
- [`src/control.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/control.rs)
- [`src/midi.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/midi.rs)
- [`src/jack.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/jack.rs)
- [`src/ui.rs`](https://github.com/PaolaShultz/shr-daw/blob/main/src/ui.rs)
- [`config/shsynth.conf`](https://github.com/PaolaShultz/shr-daw/blob/main/config/shsynth.conf)
- [`docs/AUDIO_GRAPH.md`](https://github.com/PaolaShultz/shr-daw/blob/main/docs/AUDIO_GRAPH.md)
- [`docs/CONTROLLER_INTERFACE.md`](https://github.com/PaolaShultz/shr-daw/blob/main/docs/CONTROLLER_INTERFACE.md)
- [`docs/HOW_IT_WORKS.md`](https://github.com/PaolaShultz/shr-daw/blob/main/docs/HOW_IT_WORKS.md)
- [`docs/PI5_HEADROOM_PLAN.md`](https://github.com/PaolaShultz/shr-daw/blob/main/docs/PI5_HEADROOM_PLAN.md)

### Observed instrument-host contract

SHR-DAW currently manages synthv1, Yoshimi, and FluidSynth as distinct
backends and distinct preset catalogs. Only one SHR-owned software instrument
runs at a time.

The integration model is:

```text
MIDI controller / SHR sequencer
             |
      ALSA Sequencer MIDI
             |
       managed synth process
             |
       stereo JACK outputs
             |
 direct playback or SHR audio graph
             |
       configured interface
```

Important behavior:

- Software-instrument audio is JACK-first. JACK is required for managed synth
  audio, loops, effects, and recording.
- MIDI to the managed synth uses ALSA Sequencer through `midir`.
- SHR-DAW starts and owns one external synth child at a time.
- Readiness requires one unambiguous JACK client and exactly two audio output
  ports. A MIDI port alone is not readiness.
- Exact configured client names are preferred; a single unique prefixed JACK
  client is accepted for hosts such as Yoshimi.
- SHR first establishes conservative direct playback. Its optional owned graph
  can later move the synth into the effects/final-bus path transactionally.
- Process replacement sends All Notes Off and terminates only the child SHR
  owns. Unrelated synth processes and JACK routes must remain untouched.
- Executables, client names, preset roots, MIDI selectors, and JACK ports are
  configuration rather than Rust constants.
- SHR-DAW does not start or restart JACK.
- Its JACK callback forbids allocation, locks, file access, subprocesses,
  logging, formatting, panics, and per-sample trigonometry.

### Integration decision

Moj Sint should be a fourth native managed engine with its own identity. It
must not replace `synthv1.command`, inherit synthv1 preset parsing, or pretend
that `.mojsint` files are `.synthv1` files.

The desired live executable shape is provisionally:

```sh
moj-sint --client-name shs-moj-sint --preset /path/to/file.mojsint
```

The live host should expose:

- one stable discoverable ALSA Sequencer MIDI input;
- exactly two JACK audio outputs with stable short names;
- an explicit JACK client name supplied by SHR-DAW;
- graceful SIGINT/SIGTERM shutdown;
- bounded MIDI-to-audio event transfer;
- no automatic JACK startup or restart; and
- an allocation-free, lock-free, file-free real-time callback.

The DSP core must not depend on JACK, ALSA, files, or process management. The
same core should support unit tests and deterministic offline WAV rendering on
x86_64 and AArch64.

LV2 is not recommended initially because SHR-DAW is not currently an LV2 host.
Embedding Moj Sint directly into SHR-DAW is also not recommended initially
because it breaks the established process-ownership and failure-isolation
model. Direct ALSA audio would bypass and compete with SHR's JACK graph.

Future SHR-DAW integration will require a real fourth `BackendKind`, preset ID,
catalog discovery path, configuration block, process command, Project route
identity, controller schema, pickup/reset handling, and UI labels. That work is
not part of the initial Moj Sint workspace preparation unless separately
requested.

## Performance controller contract

Controller design is a first-class synth requirement. The target physical
surface has one relative master rotary, eight sound-shaping rotary controls,
and four ADSR pots. The goal is thirteen musically useful controls, not a list
of convenient implementation parameters.

Proposed stable roles:

| Physical control | Musical role | Intent |
| --- | --- | --- |
| Master rotary | `EVOLVE` | Coherent preset-authored transformation between related sound states |
| Rotary 1 | `SHAPE` | Oscillator topology or spectral structure |
| Rotary 2 | `COLOR` | Harmonic balance and brightness |
| Rotary 3 | `EDGE` | Fold, drive, feedback, or nonlinear intensity |
| Rotary 4 | `COUPLE` | Sync, FM/PM, cross-modulation, or oscillator interaction |
| Rotary 5 | `MOTION` | Movement character or rate |
| Rotary 6 | `DEPTH` | Modulation intensity |
| Rotary 7 | `WIDTH` | Detune, phase distribution, or stereo structure |
| Rotary 8 | `SPACE` | Internal diffusion, resonant tail, or dimensional character |
| Pot 1 | `ATTACK` | Perceptually scaled onset; may coordinate amplitude and timbre |
| Pot 2 | `DECAY` | Perceptually scaled decay; may coordinate amplitude and timbre |
| Pot 3 | `SUSTAIN` | Sustained level and, where authored, sustained spectral state |
| Pot 4 | `RELEASE` | Perceptually scaled tail; may coordinate amplitude and timbre |

These are semantic anchors, not fixed one-to-one DSP parameters. A versioned
preset may map one macro to multiple internal parameters with explicit minimum,
maximum, curve, polarity, smoothing, and safe gain/energy compensation. For
example, `EDGE` may increase fold depth, adjust feedback damping, and reduce
output gain together.

`EVOLVE` must produce a broad, coherent, preset-specific transformation. It
must not be a disguised volume control. The master encoder is relative, so SHR
must accumulate and retain a normalized macro value rather than treating its
messages like absolute knobs.

The four ADSR controls remain predictable even when they coordinate amplitude
and timbre envelopes. Time controls need perceptual/exponential mappings rather
than linear milliseconds.

### SHR-DAW controller implications

Current SHR-DAW behavior discovered during inspection:

- The 12 continuous mappings in `src/control.rs` are explicitly synthv1-only.
- They use pickup after preset load/reset and currently refer to verified
  synthv1 parameter indices.
- The master rotary is translated into internal `EncoderAction` UI events and
  is not forwarded to managed synths.
- On Playback, N00B mode temporarily uses the master rotary to choose a scale.
- With N00B off, Playback rotary turns currently have no competing sound
  action, making contextual Moj Sint support feasible.

Proposed Moj Sint behavior in a later SHR-DAW change:

- On Moj Sint Playback, the master rotary controls `EVOLVE`.
- Encoder press restores the preset's saved `EVOLVE` value.
- While N00B is active, the master rotary temporarily keeps its existing scale
  role.
- Disabling N00B restores `EVOLVE` control without losing its stored value.
- The other 12 controls use pickup after preset load, reset, or Idea restore.
- Moj Sint receives its own stable macro CC/schema; it never inherits synthv1
  parameter indices or semantics.

### Control-usefulness gate

A factory preset is incomplete if any exposed control is decorative or nearly
inert. Each macro should affect most of its physical travel and remain useful
at ordinary notes, velocities, and held durations.

Automated rendering should detect at least:

- output unchanged beyond numerical noise;
- almost all change compressed into a tiny part of controller travel;
- non-finite or unbounded output;
- unintended silence at ordinary positions;
- unsafe level jumps or discontinuities;
- missing/incompatible parameter routes; and
- smoothing failure under rapid movement.

Useful measurements include waveform difference, RMS, peak, spectral centroid,
harmonic distribution, modulation depth, and envelope timing. These only prove
that a control changes output; they do not prove musical usefulness.

Every factory preset later needs a low-level human listening pass across
several notes, velocities, held durations, and polyphonic phrases. The user is
the final judge of whether the full physical travel is musically worthwhile
and whether the stable label matches what is heard.

## Initial software direction

Keep the first implementation bounded:

- a pure Rust DSP library;
- a small headless host binary;
- deterministic offline WAV rendering;
- one simple reference oscillator before exotic oscillator work;
- preallocated voices, events, buffers, modulation routes, and DSP state;
- `f32` internal audio initially;
- a portable scalar implementation first;
- optional x86_64/AArch64 optimization only behind measured, compile-time
  gates; and
- native builds on both machines as the truth, with cross-compilation checks as
  supporting evidence rather than a substitute for a native Pi build.

Suggested focused modules, subject to the approved implementation plan:

```text
src/lib.rs              public engine boundary
src/engine.rs           voices, events, block rendering
src/control.rs          13 macros, prepared routing, smoothing
src/envelope.rs         perceptual ADSR behavior
src/dsp/mod.rs          finite guards and DSP primitives
src/dsp/oscillator.rs   first reference oscillator
src/preset.rs           strict versioned .mojsint parsing/validation
src/offline.rs          deterministic rendering
src/host/jack.rs        JACK lifecycle and callback boundary
src/host/midi.rs        ALSA Sequencer input and bounded event handoff
src/main.rs             headless CLI and shutdown
```

Do not lock in this exact file map without checking what the approved design
and first tests require.

## Research backlog

After the repository and testable foundation exist, perform a structured web
investigation using authoritative or primary sources where possible. Preserve
links and references in project docs. Topics include:

- oscillator aliasing and band-limited waveform generation;
- PolyBLEP, minBLEP/minBLAMP, BLIT, wavetable, and additive methods;
- phase distortion, hard sync, FM/PM, and feedback oscillators;
- wavefolding and antiderivative antialiasing;
- nonlinear filters and virtual-analog modeling;
- coupled oscillators and chaotic systems;
- physical modeling and feedback-delay networks;
- modulation-matrix and macro-control design;
- perceptual parameter scaling and control ergonomics;
- denormals, parameter smoothing, DC blocking, and numerical stability;
- JACK real-time callback requirements;
- ALSA Sequencer MIDI; and
- Raspberry Pi 5, AArch64, and real-time Linux audio behavior.

Independently implemented Rust may use published equations and topologies with
proper provenance. Do not copy third-party DSP source code. Record licensing
and redistribution boundaries for every code, preset, sample, or other asset.

Also investigate maintained Codex skills, MCP servers, and command-line tools
for Rust DSP, source curation, persistent project knowledge, benchmarking,
dependency auditing, and Raspberry Pi deployment. Do not install speculative
or abandoned integrations merely because they exist.

Potential skill gaps identified so far:

- Rust real-time audio and DSP engineering;
- Raspberry Pi audio deployment and latency measurement;
- synth research/source curation;
- reproducible DSP benchmarking; and
- listening-test and sound-design experiment protocols.

## Design and workflow state

The architecture above was presented in chat and then revised at the user's
request to include the controller target. The revised high-level architecture
is accepted. There is no unresolved design-approval blocker. The fresh session
should proceed without reopening settled questions such as installation
authorization, ALSA-versus-JACK output, external-process integration, or the
13-control target.

The initial plan and design are recorded under `docs/superpowers/`. The
foundation fixed module and preset boundaries. The JACK crate-versus-dynamic-FFI
decision, CC numbers, useful macro mappings, and measurable usefulness
thresholds remain later engineering decisions, not broad design blockers.

The settled high-level choices are:

- external managed process;
- JACK stereo audio;
- ALSA Sequencer MIDI;
- pure/testable DSP core;
- fourth distinct SHR-DAW engine;
- 13 musically meaningful performance controls;
- Raspberry Pi 5 / 2 GB / 64-bit OS primary target; and
- x86_64 Linux development compatibility.

## Fresh-session continuation prompt

Use this short prompt after resetting; this handoff contains the detailed
context and should not be copied back into the new prompt:

```text
Continue Moj Sint in `/home/shome/p/moj-sint` from the current clean `main`.
Record `git rev-parse --short HEAD` before changing files.

Read docs/HANDOFF.md completely first and use it as the current integration,
controller, environment, research, and workflow context. The development
machine already has `libasound2-dev`; verify rather than reinstall it. Recheck
the live PaolaShultz/shr-daw main branch only if an actively changing contract
is directly relevant.

Use the existing design, architecture, host contract, research register, and
tests. Build the next smallest musical milestone test-first: implement a fair
PolyBLEP discontinuity-corrected oscillator versus integrated-wavetable
comparison, measure alias energy, peak/RMS, DC, determinism, finiteness, and
render-path allocation behavior, then select the better fit from evidence.
Route the selected oscillator meaningfully across most of `SHAPE` and `COLOR`
travel with smoothing and safe level compensation. Generate clearly named WAV
listening renders across several notes and macro positions for the user.

Keep Moj Sint a fourth external SHR-DAW instrument with JACK stereo audio,
ALSA Sequencer MIDI, and its own preset/control identities. Design around the
13-control performance contract in the handoff, including measurable macro
usefulness and later human listening acceptance.

Do not implement the live host, modify SHR-DAW, start JACK, or connect hardware
in this milestone. Preserve the allocation-free DSP boundary and strict preset
identity. Do not claim Pi performance or musical usefulness without native and
human evidence. Finish with fresh formatting, tests, Clippy, release-build,
audit, AArch64 compile, deterministic-render evidence, documented measurements,
and listening artifacts for the user to judge.
```

## Next action

Listen across the named oscillator artifact matrix. Do not call `SHAPE` or
`COLOR` musically useful until that pass succeeds. Any requested adjustment
should remain test-first and rerun the same comparison/listening evidence. Do
not rediscover SHR-DAW's established JACK/ALSA/process contract.

## Executed foundation checkpoint

Completed later on 2026-07-22 and merged into local `main`:

- initialized Git and recorded the accepted design and implementation plan;
- installed user-local stable Rust, rustfmt, Clippy, cargo-audit, and cargo-deny;
- created a pure Rust library, strict version-1 `.mojsint` preset, thirteen
  stable control identities, reference sine/ADSR, fixed voice engine, and
  deterministic stereo float WAV renderer/validator CLI;
- enforced no allocation inside `Engine::render_block` in tests;
- rechecked SHR-DAW main at
  `8b7d0d7c17c582292ac06a915ca1fe750d77bc40`;
- documented architecture, live-host contract, portability/Pi validation,
  primary-source research, licensing, and repository-specific instructions.

That checkpoint's proposed next milestone—the PolyBLEP versus integrated-
wavetable comparison and first `SHAPE`/`COLOR` route—is recorded below as
complete from automated evidence. The live JACK/ALSA host remains a separate
milestone. `libasound2-dev` is installed on the development machine; it will
still need installation on a fresh Raspberry Pi OS image.

Fresh foundation verification evidence:

- `cargo fmt --check`: exit 0;
- `cargo test --all-targets --all-features`: 21 passed, 0 failed;
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0;
- `cargo build --release`: exit 0;
- `cargo audit`: scanned 35 crate dependencies with no reported vulnerability;
- `cargo deny check`: advisories, bans, licenses, and sources passed; it reports
  one accepted duplicate-version warning for `winnow` 0.7/1.0 inside `toml`;
- `cargo check --target aarch64-unknown-linux-gnu`: exit 0 (compile evidence
  only, not a native Pi build);
- two independent reference renders were byte-identical at SHA-256
  `306e7af0f5ddf9a2fffad3a5b040ae77963244206d8bd155e4ad1fe6d11d92bc`.

## Bandlimited oscillator checkpoint

Completed later on 2026-07-22:

- implemented independent PolyBLEP and generic integrated-wavetable candidates
  over the same corrected saw-to-square target family;
- measured coherent note/shape matrices against finite band-limited Fourier
  references, with peak, RMS, DC, finiteness, exact sample hashes, and
  conservative alias/error residual energy;
- selected the integrated wavetable from aggregate residual (-25.586 dB versus
  -25.188 dB) after worst cases tied within the documented 0.1 dB window;
- added sample-path and full-engine allocation guards for both candidates and
  rapid macro events;
- routed `SHAPE` through the corrected morph and `COLOR` through a tracked
  harmonic-dark to direct-bright path, both with 10 ms smoothing and bounded
  level compensation;
- generated 27 stereo 32-bit float WAVs at 48 kHz for notes 36/60/84 and the
  complete low/mid/high `SHAPE`/`COLOR` matrix; listening-render peak spans
  0.132612 to 0.178935, RMS spans 0.046470 to 0.110782, and maximum absolute DC
  is 0.000082385; and
- did not implement the live host, touch SHR-DAW/JACK/hardware, add SIMD, or
  make Raspberry Pi or human musical-usefulness claims.

Fresh oscillator-milestone verification evidence:

- `cargo fmt --check`: exit 0;
- `cargo test --all-targets --all-features`: 27 library and 4 CLI tests passed,
  0 failed;
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0;
- `cargo build --release`: exit 0;
- `cargo audit`: scanned 35 crate dependencies with no reported vulnerability;
- `cargo deny check`: advisories, bans, licenses, and sources passed, retaining
  the accepted `winnow` 0.7/1.0 duplicate warning inside `toml`;
- `cargo check --target aarch64-unknown-linux-gnu`: exit 0 (compile evidence
  only, not a native Pi build); and
- two fresh independently generated 27-WAV matrices plus manifests had no hash
  diff; their sorted SHA-256 evidence aggregates to
  `aa8458f0209eb8d63ffa3b44c71a7d21d09e295e5fd5aeb52d86d8a8f58b705b`.
