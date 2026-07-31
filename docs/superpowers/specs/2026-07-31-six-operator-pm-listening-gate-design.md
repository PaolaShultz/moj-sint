# Six-Operator PM Listening Gate Design

**Date:** 2026-07-31
**Status:** Approved design; implementation not started
**Scope:** Clean-room DX7-informed research architecture and a six-file listening gate

## Goal

Build an independently authored six-operator phase-modulation research engine
that can reach recognizable FM categories and then turn the same principles
toward original Moj Sint sounds. The first result is an isolated, deterministic
offline listening gate. It does not replace the production Model D voice, alter
the live host, change preset schema 3, or claim Yamaha compatibility.

The listening gate answers two questions:

1. Can the implementation produce convincing category-authentic bell,
   electric-piano/mallet, and brass/bass behavior without copied patches?
2. Can three genuinely different operator graphs move beyond those calibration
   sounds into distinctive Moj Sint voices without becoming noise, generic
   distortion, or minor variants of one tone?

Human listening decides both questions. Automated measurements establish only
engineering properties such as determinism, bounds, pitch evidence, sideband
behavior, alias/error, and cost.

## Evidence and clean-room boundary

The design uses functional facts from primary or authoritative sources:

- Yamaha's official
  [DX7 operating material](https://usa.yamaha.com/support/manuals/index.html?c=music_production&k=dx7&l=en)
  documents six operators, frequency
  ratios, operator envelopes, 32 connection algorithms, multiple carriers,
  feedback, key scaling, velocity response, pitch-envelope behavior, and LFO
  control.
- John M. Chowning's 1973
  [paper](https://secure.aes.org/forum/pubs/journal/?elib=1954) and
  [US patent 4,018,121](https://patents.google.com/patent/US4018121A/en)
  describe audio-rate
  carrier/modulator relationships, time-varying modulation index, and the use
  of rational or inharmonic ratios to control evolving spectra. The US patent
  record is expired.
- [EU Directive 2009/24/EC](https://eur-lex.europa.eu/legal-content/hr/ALL/?uri=CELEX%3A32009L0024)
  distinguishes protected program expression from the
  underlying ideas and principles, including logic and algorithms.

This is an engineering boundary, not a formal legal opinion. The project will:

- write all Rust code, graph data, tables, parameter laws, scores, patches,
  tests, prose, and reports independently;
- use independently generated sine-table values and original parameter sets;
- not inspect or copy Yamaha firmware, ROM dumps, source code, voice/cartridge
  data, manuals' prose or artwork, panel appearance, or branding;
- not inspect or copy third-party emulator implementations during this work;
- not bundle factory SysEx data or derive the listening patches from it; and
- describe the work as a clean-room six-operator PM experiment, using `DX7`
  only where historically or technically necessary and never to imply Yamaha
  affiliation or product compatibility.

SysEx import, bit-exact hardware emulation, factory-patch matching, and public
compatibility claims are separate future scopes requiring their own source and
licensing review.

## Approaches considered

### Exact historical emulator first

Implement Yamaha parameter numbering, SysEx, fixed-point quirks, envelope
timing, lookup behavior, feedback quantization, and DAC-era output from the
start. This could eventually reproduce hardware closely, but it expands the
first experiment into compatibility and forensic emulation. It also increases
the risk of importing protected assets or mistaking undocumented behavior for
the musical value of the architecture. Rejected for this gate.

### Three bespoke sound generators

Hardwire one bell, one electric piano, and one brass/bass graph. This is the
fastest route to WAVs, but it would repeat the project's earlier problem of
building disposable tone-specific machinery without proving a coherent
instrument boundary. It would also make later graph, control, and polyphony
work harder. Rejected.

### Generic six-operator core with a narrow first gate

Build one validated, fixed-capacity six-operator PM voice and a declarative
algorithm table capable of representing the classic 32 routing shapes. Exercise
only three structurally different graphs in the first listening batch. This
retains architectural truth without making compatibility or 32-algorithm
audition breadth part of the first human decision. Selected.

## Architecture

The research path has four bounded units:

```text
authored patch -> validated/prepared six-op voice -> fixed score -> WAV + reports
                       |                              |
                       +-> scalar sample path         +-> human listening
                       +-> high-rate reference        +-> deterministic replay
```

### Operator

Each of six fixed operator slots owns:

- phase and prepared phase increment;
- an independently generated interpolated sine lookup;
- ratio or fixed-frequency mode;
- output level and velocity sensitivity;
- left/right keyboard level scaling around a break point;
- a four-stage rate/level amplitude envelope;
- detune small enough to preserve the intended pitch relationship; and
- reset state.

An operator emits a sine whose phase is offset by the sum of its incoming
modulation signals. Operator amplitude controls both audible carrier level and
modulation index when that operator feeds another operator. This makes spectral
evolution arise from the operator envelopes rather than from a post-filter.

Rates, levels, key scaling, ratios, increments, and smoothing coefficients are
validated and prepared at construction or note/control events. The sample path
does not allocate, lock, perform I/O, log, format, panic, access clocks, or
perform per-sample transcendental setup.

### Algorithm graph

An algorithm is immutable declarative data containing:

- directed modulation edges between the six operators;
- the carrier set summed to audible output;
- at most one declared one-sample-delayed feedback edge, which may be
  self-feedback or may close a bounded multi-operator cycle;
- a topological evaluation order for non-feedback edges; and
- a fixed output compensation value.

Validation removes the declared delayed feedback edge before deriving the
evaluation order, then rejects duplicate edges, invalid operator indices,
carrierless graphs, any remaining cycle, invalid evaluation order, and
feedback outside its declared bounds. This represents both ordinary
self-feedback and the documented classic graph whose delayed edge closes a
three-operator loop. The table may encode all 32 classic routing shapes as
connectivity facts, but the project assigns Moj Sint-owned identifiers and does
not reproduce Yamaha diagrams or UI numbering as product presentation.

The three first-gate graphs are selected because their signal flow differs,
not because their parameter values differ:

1. **Inharmonic stack:** a deep serial modulation stack with a separate short
   attack branch and one carrier. It supports bell decay and fractured-metal
   evolution.
2. **Parallel mallet:** two carrier/modulator branches plus a quiet direct
   carrier. It supports struck electric-piano body, tine motion, and a more
   original glass/wood response.
3. **Feedback branch:** one bounded feedback modulator feeding a branched
   carrier structure. It supports brass/bass edge and an original mechanical
   stab without relying on clipping.

### Voice and score renderer

The isolated research voice owns the six operators, pitch envelope, one slow
LFO, feedback history, note/velocity state, and final fixed gain. It remains
independent of JACK, ALSA, files, processes, clocks, and the production
`Engine`.

The lab owns fixed-capacity research polyphony and schedules deterministic note
events. A voice is reset before reuse; voice stealing is deterministic. The
initial listening files may use up to four offline voices where the musical
category needs chords, but this is not production or Raspberry Pi polyphony
evidence.

Pitch envelope and LFO are part of the architecture but remain subtle in the
first gate. The reference sounds must demonstrate operator-envelope spectral
motion, not hide weak synthesis behind vibrato. The original sounds may use
slightly stronger movement while retaining stable musical pitch.

### Error handling

Construction returns explicit errors for invalid sample rate, note frequency,
operator parameters, envelope stages, graph topology, feedback, gain, or score
events. Invalid state never enters the sample path. Runtime finite guards turn
an unexpected non-finite internal result into silence for that sample while a
non-real-time diagnostic records the failing configuration during tests and
sweeps.

## Listening gate

The lab writes exactly six primary 48 kHz two-channel float WAV files under an
explicit ignored `artifacts/` directory. The first gate is dry and dual-mono;
stereo processing is outside this synthesis question.

| Pair | Category-authentic calibration | Original Moj Sint development |
| --- | --- | --- |
| Inharmonic stack | Bell/metal | Fractured metal |
| Parallel mallet | Electric piano/mallet | Glass/wood |
| Feedback branch | Brass/bass | Mechanical stab |

The two files in each pair use the same deterministic category-appropriate
score and duration. Bell/metal uses velocity-varied strikes across registers;
electric piano/mallet includes single notes and a short chord; brass/bass uses
a compact low/mid phrase with accented stabs. A README gives one unambiguous
pairwise listening order.

Each patch is independently authored from synthesis principles. A calibration
sound succeeds when listeners recognize its broad category; it is not judged
against a named Yamaha factory voice. Each original development succeeds only
if it remains pitched and playable while sounding materially different from
its calibration partner.

The presentation uses one declared fixed gain per pair, chosen before the
release render. There is no per-file normalization, automatic mastering,
compressor, limiter, clipping, reverb, chorus, delay, or post-filter. Engineering
renders may be measured or gain-probed separately, but the six listening files
must expose the authored source honestly.

The six files are three topology pairs, not six claimed synthesis families.
The difference within each pair is a controlled sound-design comparison; the
three graphs provide the fundamentally different mechanisms required by the
project's listening policy.

## Measurements and tests

Development follows test-first implementation.

### Functional contracts

- all 32 graph descriptions validate and produce a stable evaluation order;
- only the declared delayed feedback edge may form a cycle;
- ratios and fixed-frequency modes track notes as specified;
- four-stage operator envelopes reach their levels, release, and return to
  idle deterministically;
- velocity and keyboard scaling move only their declared destinations;
- pitch envelope and LFO remain bounded and resettable;
- repeated construction and reset produce identical samples; and
- every listening patch retains measurable intended pitch.

### Real-time contracts

- operator sampling, one voice, four prepared research voices, and the complete
  fixed-event render path are allocation-free;
- sample paths remain finite and bounded under nominal patches, parameter
  corners, rapid note events, and maximum declared feedback;
- no sample-path I/O, logging, locks, process work, clocks, formatting, panic,
  or avoidable transcendental setup; and
- scalar Rust remains the only implementation until native evidence justifies
  a compile-time specialization.

### Spectral and alias/error evidence

- measure carrier retention and expected sideband energy for simple two- and
  three-operator fixtures before testing complete graphs;
- sweep representative low, middle, and high notes, rational and inharmonic
  ratios, modulation levels, envelope stages, and feedback;
- compare 48 kHz output with an independently rendered eight-times-rate
  reference using a clearly named conservative alias/error metric;
- report DC, peak, RMS, crest, maximum adjacent-sample jump, fitted pitch,
  harmonic/inharmonic energy, and ceiling contact; and
- retain severe high-note or feedback residuals as negative evidence rather
  than claiming the architecture is alias-clean.

Every candidate admitted to the six primary listening files must meet these
predeclared engineering bounds after its fixed pair gain is applied:

- finite samples, zero hard-clipping/ceiling-contact samples, and sample peak
  no greater than `0.95`;
- absolute DC no greater than `0.0002` and maximum adjacent-sample jump no
  greater than `0.25`;
- active RMS difference within each calibration/original pair no greater than
  `3.0 dB`, achieved through authored fixed gain rather than per-file
  normalization;
- the intended pitch anchor within `20 cents` when a harmonic fundamental is
  expected, or within `36 dB` of the strongest partial for intentionally
  inharmonic bell/metal patches; and
- conservative eight-times-rate `alias_error_db` no worse than `-20 dB` at
  MIDI 36 and 60 and no worse than `-12 dB` at MIDI 84 for every retained
  patch. These are exclusion floors, not an alias-clean claim.

If the 48 kHz baseline fails an alias/error floor, the first correction is to
bound ratios, indices, and feedback. A fixed oversampling factor may be added
only after a matched measurement shows that it clears the failed rows at an
acceptable scalar cost; it must not hide an uncontrolled graph.

### Reproducibility and project verification

- two fresh release generations must be byte-identical apart from explicitly
  excluded volatile timing data;
- the CLI test checks the exact six-file inventory and required reports;
- run `cargo fmt --check`, all-target/all-feature tests, Clippy with warnings
  denied, release build, `cargo audit`, `cargo deny check`, deterministic render
  comparison, AArch64 compile check, and `git diff --check`; and
- no Raspberry Pi sound-quality, callback, latency, or production-polyphony
  claim is made without a separately authorized native test of the selected
  path.

## Repository and product boundaries

This milestone adds isolated DSP, analysis, CLI, tests, and durable research
documentation only. It does not modify:

- the production `Engine` or Model D voice;
- `.mojsint` schema 3 or the seven current factory starting points;
- the twelve CC 20-31 controls or their current mappings;
- JACK, ALSA, SHR-DAW, installed services, or hardware;
- production voice count; or
- public claims of DX7 or SysEx compatibility.

Generated WAVs, manifests, tables, timing reports, and batch README files remain
ignored disposable artifacts. After human listening, record the verdict in
`docs/HANDOFF.md` and `docs/RESEARCH.md`, update the concise Moj Sint knowledge
note, run the knowledge validator, and delete the batch unless the user
explicitly requests preservation of a specific result.

## Human decision after the gate

The user listens to the three pairs at ordinary playback level. The decision is
per topology and per sound, not one global green/red metric:

- whether each calibration sound is category-recognizable;
- whether each original sound is musically useful and materially distinct;
- whether pitch, velocity response, attack, decay, and register translation
  work on headphones and speakers; and
- whether any graph deserves refinement or production/control integration.

No graph, patch, macro mapping, or production replacement is selected before
that verdict.

## Fructal cap test

1. The obvious action—render the gate—produces one six-file comparison and its
   reports, without modifying the live instrument.
2. The motion is coherent: validate and prepare, render, measure, listen, then
   decide. Compatibility, preservation, and production integration are not
   hidden side effects.
3. Legal provenance, real-time safety, the twelve-control budget, and artifact
   policy remain active exactly where needed.
4. The current Model D engine, preset state, live host, user verdicts, and
   experiment ownership remain intact.
5. Invalid configurations fail before rendering; deterministic regeneration
   and nearby reports provide recovery and diagnosis.
6. The remaining effort—authoring graphs, envelopes, patches, and listening—is
   intrinsic to learning whether six-operator PM belongs in Moj Sint.
