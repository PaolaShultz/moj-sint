# Moj Sint workspace handoff

Last updated: 2026-07-23, Europe/Zagreb.

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

Moj Sint is not permanently monophonic. Design and validate its sound engine
monophonically first so oscillator topology, routing, macro behavior, and cost
can be understood without voice-count multiplication. Expand the selected
architecture to polyphony only after native Raspberry Pi callback/headroom
measurements establish a safe voice budget.

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

### Experimental intent and rapid idea capture

The user is building Moj Sint to explore unfamiliar machine behavior, not to
make another conventional sine/saw synthesizer or to demonstrate music-theory
knowledge. The desired instrument is a machine whose physical controls produce
meaningful, discoverable responses and can reach sounds worth playing without
hiding experimental behavior behind generic presets.

Capture the user's new sound or machine ideas in this handoff or
`docs/RESEARCH.md` promptly, even when they arrive as incomplete metaphors,
rough implementation thoughts, or apparently incompatible mechanisms. Do not
require the user to translate an idea into established synthesis terminology,
and do not silently normalize unusual ideas into familiar subtractive-synth
patterns. Preserve the strange premise, separate what is known from what is
speculative, and turn it into the smallest bounded experiment that could prove
or reject it.

Think from the machine outward as well as from acoustics or music theory
inward. Registers, integer overflow, shifts, rotates, carry, quantization,
lookup addressing, state-machine cycles, memory layout, connection order, and
cheap parallel state are legitimate sources of sound hypotheses. Mathematical
or psychoacoustic analysis should help bound and understand those hypotheses;
it should not automatically replace them with textbook oscillator designs.

## Experimental-output and variation policy

This policy applies specifically to sound-research experiments. Generated
audio, measurement tables, manifests, batch-specific reports, parameter files,
and similar experiment outputs are disposable by default. Generate them under
the ignored `artifacts/` tree or outside the repository. When an experiment is
rejected, record its conclusion and limitation in the durable research/handoff
documents, then delete the generated batch rather than archiving it.

If a batch, individual tone, experiment-specific document, parameter/preset
file, or other output appears worth developing or retaining, the assistant may
offer to preserve that exact material. It must not copy anything into a
separate tracked directory unless the user explicitly requests preservation.
No preservation directory should be created speculatively. This restriction
does not apply to ordinary source code, tests, or concise updates to durable
project documentation.

Listening experiments must compare fundamentally different sound-generation
hypotheses: for example, a phase-structured multi-oscillator source versus a
noise-excited resonator versus another genuinely distinct topology. Different
parameter values, drive amounts, filtering strengths, wet levels, or several
closely related saw/sine variants do not count as different sounds. Automated
parameter sweeps remain useful for bounds, aliasing, stability, and cost, but
must not be presented as the musical-variation listening gate.

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
- The oscillator listening matrix was rejected as insufficiently varied. All
  generated milestone artifacts have been removed from the workspace; only
  the documented measurements and disposable generators remain.
- The rejected harmonic selector has been retired from the engine render path.
  The subsequent per-voice parallel character layer also failed listening: it
  presented the same weak source under similar treatments rather than distinct
  sound identities. It is not an accepted factory sound or macro mapping.
- The foundation engine can preallocate multiple voices, but this is not a
  final polyphony commitment. The next oscillator-system design and listening
  work is explicitly monophonic; native Raspberry Pi evidence gates later
  polyphonic expansion.
- User-local stable Rust 1.97.1, Cargo, rustfmt, Clippy, cargo-audit, and
  cargo-deny are installed.
- The live JACK/ALSA host is not implemented and no audio service or hardware
  was touched.
- The approved five-family comparison is implemented as isolated disposable
  offline research code. The user found the direction promising but still far
  from the intended instrument; no family is selected and none is routed into
  `Engine` or stable macros. The reviewed 15-file batch was deleted under the
  experimental-output policy.
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

## Voice-count strategy

The musical architecture is **mono-first, poly-later**, not mono-only:

- During oscillator research, routing experiments, macro mapping, and initial
  listening, render exactly one active musical voice. This makes phase,
  feedback, nonlinear gain, smoothing, and CPU cost attributable.
- Do not use the existing `voices = 8` foundation preset as evidence that eight
  complex voices are affordable. It only proves fixed preallocated voice
  storage for the simple current engine.
- Keep oscillator/DSP state structurally per-voice and keep the engine boundary
  capable of fixed polyphony; do not introduce global singleton DSP state that
  would make later expansion unsafe.
- Before expanding, run native Raspberry Pi callback timing under representative
  notes, macro motion, and worst-case topology. Establish a conservative budget
  with headroom for JACK, MIDI/event transfer, and the rest of SHR-DAW.
- Add voices incrementally and repeat allocation, deadline, determinism,
  finiteness, level, and listening checks. Polyphony may also need per-voice
  phase/seed policy, voice stealing, mix compensation, and reduced internal
  topology; none is selected yet.
- Do not promise a voice count until native evidence exists. Monophonic design
  is a sequencing decision that protects sound research, not a product lock.

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

The first broader oscillator-system survey is now recorded in
`docs/RESEARCH.md` under “Distinctive oscillator systems and routing survey.”
It covers Moog, Prophet-5, Buchla, DX7, Casio CZ, PPG, JP-8000, higher-order
FM, nonlinear modeling cost, and coupled oscillators, then converts the user's
intentionally speculative ideas into seven testable Moj Sint experiment
families. Continue from that register rather than reducing the next phase to
vintage emulation.

## Next experimental phase: five-family listening gate

The user approved a genuinely varied listening gate built from five separate
sound-generation hypotheses. These are not five parameter settings in one
graph, and the first implementation should not force all five into the
production `Engine`. Prototype them as disposable monophonic offline research
sources first:

1. **Nonlinear PM/complex oscillator.** Shape or fold a modulator before it
   drives a bounded phase-modulated carrier. Connection order, ratio, phase,
   DC control, and sideband behavior are part of the source identity; this is
   not another processed saw or sine bank.
2. **Excited comb/resonant source.** Excite tuned or deliberately inharmonic
   feedback combs with an impulse, noise burst, or another short excitation.
   Here the delay network generates the sustained tone. Compare pitch-locked,
   dispersed, and safely damped structures as engineering sweeps, but present
   the family as one listening candidate rather than pretending that several
   delay lengths are different source mechanisms.
3. **Psychoacoustic micro-delay spatial-motion source.** Use several very
   short, unequal, slowly changing delay paths to create controlled
   decorrelation, apparent space, and a sensation that energy moves or
   pulsates across the middle of the image or spectrum. Investigate precedence
   effect, comb coloration, interaural time/level relationships, mid/side
   energy, mono compatibility, headphone/speaker translation, and modulation
   audibility. It must have its own perceptual hypothesis and must not collapse
   into an ordinary chorus, flanger, ping-pong echo, or wet/dry sweep.
4. **Spectral-traversal source.** Traverse a small Moj Sint-authored family of
   related spectra or phase laws as one coherent identity. It must not become
   an arbitrary waveform browser or copy a commercial wavetable or preset.
5. **Small-register integer machine and oscillator swarm.** Build sound from
   explicitly sized integer phase/state registers, wrapping arithmetic,
   shifts, rotates, carry/borrow relationships, bit selection, and other
   bounded state-machine operations. For a fixed-point phase increment, a
   left shift can represent a power-of-two frequency relationship such as an
   octave until the defined-width boundary is reached; the boundary behavior
   must be deliberate rather than accidental overflow. Explore whether small
   registers, short cycles, quantization, and many extremely cheap interacting
   integer voices create useful controlled nastiness. This is a sound-source
   hypothesis, not merely a premature optimization exercise.

The comb/resonant and psychoacoustic micro-delay candidates are deliberately
separate. In the comb family, delay is the resonating sound generator. In the
psychoacoustic family, multiple sub-echo delays organize spatial perception
and internal motion. Sharing a delay-line primitive does not make the two
topologies equivalent.

Before implementation, research each family from primary or authoritative
sources and extend `docs/RESEARCH.md` with authorship, publication details,
direct URLs, licensing implications, and the specific design claim supported
by each source. Define a concise perceptual hypothesis for every candidate:
what the listener should recognize, why it differs structurally from the
other four, and what failure would sound like.

The first listening batch should contain one bounded representative of each
family, loudness matched across representative low, middle, and high notes.
Automated sweeps within a family remain engineering evidence only. They test
stability, useful travel, pitch behavior, aliasing, modulation, mono
compatibility, and cost; they do not count as additional musical variations.
The user decides whether the five candidates actually sound distinct and
worth developing before any one is selected for engine integration or macro
mapping.

For the integer-machine family, use portable scalar Rust with explicit
fixed-width types and defined wrapping operations first. Do not introduce
inline assembly, SIMD, architecture-specific behavior, or undefined overflow
semantics merely because the idea began from an assembly metaphor. Convert to
the floating-point audio boundary in a controlled place and measure DC,
periodicity, pitch error, level, spectral distribution, aliasing, deterministic
replay, and the audible consequences of register-width and state-cycle
changes. Different bit widths and oscillator counts are engineering sweeps
inside this one family, not separate listening mechanisms.

The hoped-for payoff is enough cheap parallel machines to support an eventual
four-note polyphonic instrument with rich internal oscillator populations.
“Zillions of oscillators” is a productive hypothesis, not a performance claim.
Research remains monophonic first; only native Raspberry Pi callback/headroom
measurements may establish how many machines per voice and whether four
simultaneous notes are actually safe.

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
- mono-first oscillator-system design with polyphony deferred until native Pi
  cost/headroom evidence, without permanently locking the engine to mono;
- Raspberry Pi 5 / 2 GB / 64-bit OS primary target; and
- x86_64 Linux development compatibility.

## Fresh-session continuation prompt

Use this short prompt after resetting; this handoff contains the detailed
context and should not be copied back into the new prompt:

```text
Continue Moj Sint in `/home/shome/p/moj-sint` from the current clean `main`.
Record `git rev-parse --short HEAD` before changing files.

Read `docs/HANDOFF.md` and `docs/RESEARCH.md` completely before planning or
changing files. Treat the existing integration contract, 13 controls,
bandlimited-oscillator evidence, and source/licensing register as settled
context. Do not recheck SHR-DAW unless an actively changing host contract is
directly relevant.

This phase is mono-first sound research, not a permanent monophonic lock and
not yet a polyphony milestone. The shared-phase harmonic selector and parallel
character layer already failed human listening because they sounded like weak
versions of one basic source. Do not revive them through parameter tweaks,
additional output processing, or batches of related saw/sine variants.

Design a disposable offline listening gate with one bounded representative
from each of the five approved families in “Next experimental phase:
five-family listening gate”: nonlinear PM/complex oscillator, excited
comb/resonant source, psychoacoustic micro-delay spatial-motion source,
spectral traversal, and a small-register integer machine/oscillator swarm.
Research all five first and state a distinct perceptual hypothesis for each.
Keep the prototypes isolated from the production Engine until the user has
listened; do not implement five permanent architectures at once or clone a
legendary synth.

Keep the DSP scalar, deterministic, finite, smoothed, and allocation-free.
Use test-first development for every render path. Measure alias/error where
oscillation or nonlinearity creates it, harmonic/spectral distribution,
peak/RMS/DC, pitch retention, useful control travel, rapid movement,
determinism, allocation, and workstation cost. For the micro-delay candidate,
also measure mono fold-down, inter-channel correlation, side-to-mid energy,
delay/modulation bounds, and discontinuity-free delay movement. Generate
clearly named, loudness-matched low/middle/high-note listening artifacts and
leave all musical usefulness and spatial-effect acceptance to the user.

For the integer-machine candidate, define register widths and every overflow,
shift, rotate, and conversion rule explicitly. Test finite/bounded output,
repeat periods and accidental lockups, pitch relationships, DC, aliasing,
determinism, allocation, and cost. Start in portable scalar Rust. Do not add
assembly, SIMD, or architecture gates until measurement proves a need.

Several comb lengths, modulation rates, delay offsets, wet levels, or stereo
widths do not count as separate listening variations. The comb network must
act as a source; the psychoacoustic network must aim at controlled space and
mid-image or midband pulsation without becoming an ordinary chorus, flanger,
or echo. Verify the spatial candidate on headphones, speakers, and mono only
when the user explicitly performs or authorizes that listening setup.

Preserve per-voice state so later polyphony remains possible, but do not expand
voice count or claim a safe count until the selected complex topology is
benchmarked natively on the Raspberry Pi with callback/headroom evidence. Do
not implement the live JACK/ALSA host, modify SHR-DAW, start JACK, connect
hardware, add SIMD, copy third-party DSP/presets/samples, or claim Pi
performance in this phase. Finish documentation and the repository verification
matrix required by `AGENTS.md`. Keep every generated batch under ignored
`artifacts/` or a fresh temporary directory. After the user's verdict,
document the conclusion and delete the batch unless the user explicitly asks
to preserve a specific result.
```

## Next action

Reset into the continuation prompt above, research the five approved source
families, and build the smallest disposable monophonic comparison that gives
the user one honest representative of each. Stop for human listening before
selecting a family, integrating it into `Engine`, or assigning stable macros.
The existing `SHAPE`/`COLOR` listening gate remains open, so do not call those
routes musically useful without user acceptance. Do not rediscover SHR-DAW's
established JACK/ALSA/process contract.

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
- generated, measured, and later removed 27 stereo 32-bit float WAVs at 48 kHz
  for notes 36/60/84 and the complete low/mid/high `SHAPE`/`COLOR` matrix;
  listening-render peak spans
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

## Shared-phase harmonic-selector checkpoint

Completed later on 2026-07-22:

- compared nonlinear modulator preprocessing, a shared three-phase selector,
  and a small directed operator graph, then selected the shared 0/120/240-degree
  bank as the smallest bounded connection-order experiment;
- implemented one scalar per-voice recursive phase source whose algebraic taps
  cancel linearly and whose normalized equal cubic sum isolates the third
  harmonic, without per-sample trigonometric setup;
- routed smoothed `EDGE` from the selector fundamental to its third harmonic and
  smoothed `COUPLE` from the existing oscillator to the selector, preserving the
  thirteen stable roles and the unchanged `COUPLE=0` baseline;
- added exact tap/cancellation, deterministic reset, finite/bounded output,
  high-note third-harmonic guard, macro-travel, rapid-movement, and
  oscillator/engine allocation tests;
- generated 27 loudness-matched one-voice listening conditions for notes
  36/60/84 and low/middle/high `EDGE`/`COUPLE`, with harmonic, pitch-retention,
  residual alias/error, peak/RMS/DC, and deterministic-hash evidence;
- measured the selector through MIDI 127 and tapered its third harmonic between
  40% and 48% of sample rate so the route returns to the fundamental before the
  generated partial crosses Nyquist;
- recorded a separate volatile release workstation micro-timing for the
  isolated selector, without treating it as callback or Raspberry Pi evidence;
  and
- left human musical-usefulness acceptance open at generation time and made no
  Pi, latency, polyphony, or sound-quality claim.

The design and plan are in `docs/superpowers/`. The pure third-harmonic endpoint
intentionally removed the fundamental, while the rest of the matrix retained it
under the recorded test rule.

The user completed the human listening pass and rejected all 27 conditions as
too close to pure sine material and not usefully distinct. The generated
`artifacts/harmonic-selector-milestone/` directory was removed after review;
the lab generator remains only for reproducible temporary test evidence. The
experimental `EDGE`/`COUPLE` route is therefore a negative research result, not
an accepted macro mapping or factory sound. The next phase should keep a stable
base sound and investigate controlled parallel output/character layers such as
bounded saturation, harmonic excitation, subharmonic reinforcement, or
feedback/resonant processing, comparing per-voice and post-mix placement rather
than stacking every layer serially.

## Parallel character-layer checkpoint

Completed later on 2026-07-22:

- compared a full-band symmetric saturation return, a band-limited symmetric
  generated-residual return, and a band-pass asymmetric exciter; separately
  deferred per-voice oscillator, divider, and tracked subharmonic forms;
- selected one per-voice branch before mixing, preserving the existing
  `SHAPE`/`COLOR` output as an untouched dry anchor and avoiding post-mix
  inter-voice products;
- implemented a four-pole 6 kHz branch low-pass, bounded cubic soft clip,
  linear-component subtraction, 10 Hz DC blocker, and two-stage note-tracked
  return high-pass, all with scalar preallocated per-voice state;
- retired the rejected selector from `Voice` while preserving its standalone
  deterministic negative-result generator and primitive tests;
- kept `EDGE` as candidate drive/intensity and `COUPLE` as candidate return,
  both smoothed over 10 ms, with sample-identical `COUPLE=0` bypass for every
  `EDGE` value;
- rejected two intermediate revisions that either cancelled the dry
  fundamental or measured like minor level/EQ change, then capped drive at 2x
  and retained a return that measurably lifts a broad upper-harmonic set;
- compared direct, first-order ADAA, and two-times evaluation against an eight-
  times zero-phase reference. Direct passed the declared rule at -73.700 dB
  worst moderate and -59.121 dB worst strong residual and was selected; this is
  topology-specific evidence, not a general rejection of nonlinear
  antialiasing;
- measured 600/900 Hz intermodulation at -134.381 dB for separate per-voice
  branches versus -15.463 dB for the deferred post-mix placement;
- generated exactly nine loudness-matched listening WAVs: dry, moderate, and
  strong for MIDI notes 36, 60, and 84. Peak is 0.139552-0.143008, RMS is
  0.079769-0.079980, maximum absolute DC is 0.000000196, and every condition
  retains the fitted fundamental;
- recorded 12-partial distributions, deterministic hashes, alias/error rows,
  intermodulation evidence, and volatile scalar workstation timing; and
- received a negative listening verdict: all conditions sounded like the same
  basic tone with small treatments, so neither the graph nor its
  `EDGE`/`COUPLE` mapping is an accepted factory sound.

All generated oscillator, selector, and parallel-character milestone artifacts
have been removed after review. Future evidence must be generated into a fresh
temporary directory and removed after its decision is documented. The next
experiment must improve the dry per-voice source and compare structurally
different source families rather than adding more output processing.

All cost evidence in this checkpoint comes from the scalar x86_64 workstation.
It is not Raspberry Pi callback, latency, safe-polyphony, or sound-quality
evidence. No JACK/ALSA host, SHR-DAW change, hardware connection, SIMD path, or
Pi claim was added.

## Five-family listening-gate checkpoint

Completed on 2026-07-23 and left open for human listening:

- researched nonlinear PM, excited combs, precedence/micro-delay perception,
  spectral traversal, and small-register sound machines from primary or
  authoritative sources, preserving authorship, publication details, direct
  URLs, supported claims, and licensing boundaries in `docs/RESEARCH.md`;
- defined a distinct perceptual hypothesis and audible failure condition for
  each family before implementation;
- added an isolated `dsp::research` source boundary and `five-family-lab`
  offline binary without changing the production `Engine`, presets, controller
  routes, JACK/ALSA work, or SHR-DAW;
- implemented one fixed scalar representative per family with per-source state,
  deterministic reset, finite/bounded output, and allocation-free sample paths;
- generated exactly 15 stereo 32-bit-float 48 kHz files for MIDI 36/60/84,
  matched to RMS 0.06 except the crest-limited high comb at 0.059116, with peak
  0.085294-0.979000 and maximum absolute DC 0.000229049;
- measured harmonic distributions, fitted fundamental/pitch retention,
  left/right correlation, side-to-mid energy, mono fold-down, adjacent-sample
  continuity, deterministic hashes, integer phase periods, conservative 8x
  alias/error residuals, and volatile scalar workstation cost;
- corrected two issues found by release evidence: approximate spectral rotation
  coefficients caused long-render pitch drift, and the high comb retained too
  much DC; exact prepared rotations and a prepared 10 Hz comb DC blocker now
  have focused regressions;
- retained the integer swarm's -0.658/-0.399/-0.707 dB high-rate residual as a
  documented negative result showing strong register-quantization and
  sample-rate dependence rather than calling it alias-clean; and
- produced byte-identical release WAVs and deterministic reports in two fresh
  directories, excluding only the explicitly volatile workstation timing file;
  the sorted per-file hash manifest has SHA-256
  `820f8a46f5fa8bcf39c9884b5b986196809b4cccfb7f3b328e4e67916798f572`; and
- passed fresh `cargo fmt --check`, all-target/all-feature tests (61 library,
  one five-family CLI, and six existing CLI tests), Clippy with warnings denied,
  release build, `cargo audit` over 35 crate dependencies, `cargo deny check`,
  AArch64 compile check, and `git diff --check`. `cargo deny` retains only the
  accepted `winnow` 0.7/1.0 duplicate warning inside `toml`.

The user's first listening verdict is that Moj Sint remains far from the goal,
but these fundamentally different sources finally point in the right direction.
This is not acceptance, rejection, or ranking of any family. Do not select,
integrate, expand, or map one based on this pass. The next session should keep
exploring different fundamentals under the user's guidance rather than
prematurely converging on these five representatives. The spatial candidate
has not been accepted on headphones, speakers, or mono; automated metrics
cannot make that decision. The reviewed ignored batch was deleted after the
verdict, while the reproducible generator, tests, and durable evidence remain.
