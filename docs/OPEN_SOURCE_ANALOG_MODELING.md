# Open-source analog synth modeling knowledge base

Surveyed 2026-09-09. This is a practical reading map for Moj Sint, based on
upstream repositories, manuals, and published research. It is not an exhaustive
catalog or a listening ranking. Project statements are distinguished below
from our proposed applications. The initial survey did not build external
synths; the subsequent [Open303 candidate](OPEN303_CANDIDATE.md) now provides
a reviewed core import and executable offline checks.

## Working principle

Start with what earlier designers already solved. Reconstruct a documented
signal flow, including modulation, feedback, gain placement, and note behavior.
Establish that baseline before changing its mechanism. A familiar oscillator
feeding a generic filter is insufficient evidence of a particular device model.

Keep three levels separate:

- **Device-flow emulation:** preserve the instrument's connections and control
  interactions; individual blocks can be approximations.
- **Circuit-derived modeling:** derive a block's behavior from its circuit and
  choose a numerical solution. This does not establish whole-instrument fidelity.
- **Original virtual analog:** use analog-inspired mechanisms without claiming
  a specific hardware replica.

These categories describe evidence, not a quality hierarchy. Moj Sint can learn
from all three and retain its own model identity.

## Start here

The priorities and proposed uses in this table are our research judgment.
Licenses are upstream declarations, not completed reuse clearance.

| Reference and authors | What it provides | First use for Moj Sint | License / qualification |
| --- | --- | --- | --- |
| [Open303](https://github.com/RobinSchmidt/Open303), Robin Schmidt | TB-303 emulation; original SourceForge work continued by its author | Pressure Chain: study the complete bass voice and accent/slide/filter interactions | [MIT](https://github.com/RobinSchmidt/Open303/blob/main/License.txt), copyright 2009; inspect dependencies separately |
| [OB-Xf](https://github.com/surge-synthesizer/OB-Xf), Surge Synth Team; lineage from Vadim Filatov / 2DaT and discoDSP | Continuation of the last open-source OB-Xd; Oberheim OB-X inspiration | Study a coherent polyphonic voice and controlled voice differences | GPLv3; do not assume current commercial OB-Xd releases share this source |
| [Odin 2](https://github.com/TheWaveWarden/odin2), TheWaveWarden and contributors | Complete hybrid synth with separate analog filter implementations | Compare K35, diode ladder, transistor ladder, and SEM mechanisms | [GPLv3](https://github.com/TheWaveWarden/odin2/blob/master/LICENSE); font has separate OFL terms |
| [Surge XT](https://github.com/surge-synthesizer/surge), Claes Johanson and Surge Synth Team | Hybrid instrument with documented filter configurations and extensive modulation | Dual Filter: study routing and filter interaction; use as a reference library | GPLv3; original commercial project opened in September 2018 |
| [sst-filters](https://github.com/surge-synthesizer/sst-filters), Surge Synth Team and credited upstream authors | Extracted filter library including OB-Xd, Odin-derived K35/diode, vintage ladder, warp, tri-pole | Find existing algorithm families before deriving another filter | GPLv3; raw SSE intrinsics, with SIMDe suggested for other targets |
| [Hera](https://github.com/jpcima/Hera), Jean Pierre Cimalando, crediting pendragon-andyh and dzannotti | Juno-60 emulation with MPE and linked earlier reverse-engineering work | Study a restrained complete device architecture and envelope behavior | GPLv3; upstream calls it alpha and explicitly documents inaccuracies |
| [Bristol](https://sourceforge.net/projects/bristol/), Nick Copeland | More than thirty vintage instrument emulators in one package | Broad historical map of device-specific arrangements | GPLv2 as listed upstream; historical project, not a verified modern fidelity reference |
| [amsynth](https://github.com/amsynth/amsynth), Nick Dowell and contributors | Compact subtractive VA: two oscillators, sync, resonant multimode filter, two ADSRs, mono/poly/legato | Simple baseline for voice and note semantics | [License file](https://github.com/amsynth/amsynth/blob/master/COPYING); verify exact applicable terms before reuse |
| [Helm](https://github.com/mtytel/helm), Matt Tytel and contributors | Original synth with cross modulation, oscillator feedback/saturation and modulation routing | Strange Oscillator: study how unusual mechanisms become playable controls | GPLv3; not a named analog-device replica |
| [Monique](https://github.com/surge-synthesizer/monique-monosynth), Thomas Arndt and Surge Synth Team | Original monophonic synth, opened in December 2021 | Secondary source for an expressive bass instrument's architecture | Source declared dual GPLv3/MIT; JUCE and content require separate attention |
| [VCV Fundamental / Free](https://github.com/VCVRack/Fundamental), VCV | Essential modular synth modules with exposed source | Study module boundaries and patchable synthesis primitives | [License details](https://github.com/VCVRack/Fundamental/blob/v2/LICENSE.md); source and visual assets must not be conflated |
| [MoogLadders](https://github.com/ddiakopoulos/MoogLadders), Dimitri Diakopoulos and credited algorithm authors | Collection of alternative digital ladder implementations | Model D: locate contrasting numerical models and their original sources | Per-algorithm licenses differ; root Unlicense is not blanket clearance |
| [chowdsp_wdf](https://github.com/Chowdhury-DSP/chowdsp_wdf), Jatin Chowdhury and contributors | C++ wave digital circuit-modeling library | Study a specific circuit block when its interactions justify component modeling | BSD-3-Clause declaration; optional dependencies have their own terms |

## Focused study cards

### Open303: study the bass instrument as a system

The subsequent [deep Open303 analysis](OPEN303_ANALYSIS.md) supersedes this
survey's provisional integration assessment. It traces the source, compares
JC-303, verifies the Ooura FFT provenance, and records offline C++/Rust,
sanitizer, allocation, state, finite-output, and spectral evidence. Direct
core reuse is feasible after bounded repairs; the original is not callback-ready.


Primary sources: [author's repository](https://github.com/RobinSchmidt/Open303),
[original project description](https://github.com/RobinSchmidt/Open303/blob/main/ReadMe.old.txt),
[DSP directory](https://github.com/RobinSchmidt/Open303/tree/main/Source/DSPCode).
The current README describes continuation of the author's old SVN project and
work toward a CLAP plugin; historical VST2 instructions are not current build
assurance.

Verified entry points are `rosic_TeeBeeFilter.{h,cpp}`,
`rosic_AnalogEnvelope.{h,cpp}`, `rosic_DecayEnvelope.{h,cpp}`, and
`rosic_BlendOscillator.{h,cpp}` in that DSP directory.

**Questions to answer in the next implementation study:** Where do accent and
slide enter? Which states carry between overlapping notes? How do envelope
amount, resonance, internal drive, and output level interact? Which parts are
specific emulation choices rather than circuit facts? Trace callers before
extracting any individual block.

**Our application:** compare these mechanisms with the existing
[acid-chain research](ACID_CHAIN_RESEARCH.md). Pressure Chain remains our
original instrument; finding a 303 reference does not turn it into a TB-303
replica. Keep sequencer semantics separate from filter topology.

### OB-Xf: complete voices and variation with a purpose

Primary sources: [project and authorship](https://github.com/surge-synthesizer/OB-Xf)
and [voice-variation manual](https://surge-synth-team.org/ob-xf/manual/voice-variation/).
Verified entry points in `src/engine/` are `Voice.h`, `OscillatorBlock.h`,
`Filter.h`, `AdsrEnvelope.h`, `VoiceMatrix.h`, and `VoiceQueue.h`.

**Our application:** trace oscillator mix through filtering and amplitude, then
inspect what varies per voice, when it varies, and how note allocation affects
it. Distinguish a persistent voice offset from time-varying drift. Reproduce
variation deterministically for measurements before considering randomness.
These are study questions, not claims that each behavior has been audited here.

The OB-Xf project describes OB-X inspiration; Surge's manual identifies its
OB-Xd 12 dB filter lineage with OB-Xa. Keep the actual instrument and filter
lineages explicit rather than treating every Oberheim name as interchangeable.
See the [Surge filter manual](https://surge-synthesizer.github.io/manual-xt/index.html).

### Odin 2 and Surge: follow the existing reuse chain

[Surge's extracted filter library](https://github.com/surge-synthesizer/sst-filters)
explicitly credits OB-Xd for its OB-Xd filters and Odin 2 for K35 and diode
ladder filters. This is a concrete example of prior work being carried forward
with provenance.

Verified Odin entry points in
[`Source/audio/Filters`](https://github.com/TheWaveWarden/odin2/tree/master/Source/audio/Filters)
include `Korg35Filter.cpp`, `DiodeFilter.cpp`, `LadderFilter.cpp`,
`SEMFilter12.cpp`, and `VAOnePoleFilter.cpp`. Its oscillator directory includes
`AnalogOscillator.cpp` and `DriftGenerator.cpp`.

**Our application:** compare filter structures, nonlinear element placement,
feedback closure, coefficient preparation, and sample-rate handling. Record
which differences arise from the circuit family and which from discretization.
Use the [SST API documentation](https://surge-synthesizer.github.io/sst-docs/docs/sst-filters/)
to navigate the extracted implementations. The library is an algorithm reference,
not evidence that its SIMD layout is appropriate for our scalar Rust baseline.

For Dual Filter, first draw the intended series/parallel/feedback graph using
our existing blocks. Consult Surge's documented configurations for precedent;
only add a new filter when the musical hypothesis requires one.

### Hera and Bristol: learn complete flows, retain known limitations

[Hera's README](https://github.com/jpcima/Hera) links the earlier
[Juno60](https://github.com/pendragon-andyh/Juno60) and
[junox](https://github.com/pendragon-andyh/junox) work. It documents excessive
VCF self-oscillation and unfinished envelope behavior. Verified entry points
include `Source/HeraEnvelope.cpp`, `Source/HeraLFOWithEnvelope.cpp`, and
`Source/bbd_filter.cpp`.

**Our application:** use this lineage to investigate oscillator/sub/noise mix,
filtering, envelope/gate behavior, and chorus placement in a Juno-style study.
Verify the full routing against device documentation before asserting it.
Its disclosed defects are useful regression questions, not behavior to imitate.

[Bristol's upstream page](https://sourceforge.net/projects/bristol/) describes
a broad emulator collection with a separate UI and audio engine. Use it to
find contrasting instrument architectures. Its age and emulator count do not
establish circuit accuracy, low aliasing, or real-time suitability on our Pi.

### Ladder models and circuit methods: compare the assumptions

[MoogLadders](https://github.com/ddiakopoulos/MoogLadders) collects several
approaches and their original sources. The maintainer explicitly says the
models have not been verified across all cutoff/resonance/sample-rate
combinations. Use it as a bibliography and comparison map. Preserve each
algorithm's author and license rather than attributing every model to the
collection maintainer.

[chowdsp_wdf](https://github.com/Chowdhury-DSP/chowdsp_wdf) provides circuit
components and adaptors for real-time wave digital models. The associated
publication is Jatin Chowdhury, *chowdsp_wdf: An Advanced C++ Library for Wave
Digital Circuit Modelling*, 2022,
[arXiv:2210.12554](https://arxiv.org/abs/2210.12554). The paper and library are
separate works; the library license does not automatically license figures or
text from the paper.

**Our application:** begin with the existing Moog/Huovilainen primary-source
register in [RESEARCH.md](RESEARCH.md). Reach for component-level methods only
when a named interaction cannot be represented adequately by the simpler
baseline. Compare numerical stability, aliasing, and native cost as well as
listening results. This survey does not select a solver.

## What to consult for our next task

| Moj Sint area | First reading | Concrete question |
| --- | --- | --- |
| Model D | Existing research → MoogLadders → SST vintage ladders | What changes when feedback/nonlinearity is modeled differently at the same input level? |
| Pressure Chain | Existing acid research → Open303 | Which envelope, accent, slide, and gain interactions contribute beyond the filter alone? |
| Dual Filter | Surge configurations → Odin/SST | Does the proposed graph actually change feedback or spectral interaction? |
| Strange Oscillator | Helm oscillator mechanisms → Odin oscillators | Is the new variation a different mechanism with useful control behavior? |
| Swarm / future polyphonic analog voice | OB-Xf → Hera | Which voice differences and note-state rules are musically useful? |
| A future named-device study | Bristol discovery → specific emulator → original device manual | Can we explain the entire flow and identify where our approximation departs? |

Six-Op PM belongs primarily to digital FM/PM research. [Dexed](https://github.com/asb2m10/dexed)
is a relevant DX7 reference, but is not evidence about analog circuitry.
Likewise, processor/firmware emulation such as
[The Usual Suspects](https://github.com/dsp56300/gearmulator) is a different
method from deriving an analog circuit model. Keep those categories distinct.

## Reusable investigation record

Before implementing a proposed analog model, add a short record here or in the
owning research document:

1. **Reference:** device, primary manual/paper, upstream project, author, exact
   revision studied, license file and any per-file exceptions.
2. **Flow:** original diagram written in our own notation, including audio,
   control, feedback, drive, output gain, and state carried between notes.
3. **Mechanism:** what gives this flow its identity; what earlier authors
   modeled, approximated, or explicitly left inaccurate.
4. **Baseline and change:** reproduce the useful flow first; state one
   structural modification and its intended audible consequence.
5. **Evidence:** finite-output and allocation checks; aliasing measurements
   for oscillator/nonlinear changes; control sweeps as engineering checks;
   matched musical listening for structurally different candidates.
6. **Decision:** accepted, rejected, or unresolved; why; native cost evidence
   separately from workstation evidence. Keep generated batches disposable.

Do not promote a project feature list into a fidelity claim. A source filename
is a navigation lead, not proof of its equations or behavior. This survey
checked upstream descriptions, license declarations, and selected source-tree
entry points; detailed source audits, hardware comparisons, and listening
remain future scoped work. Pin upstream revisions when that work begins.

No source, presets, samples, figures, or third-party prose were imported into
Moj Sint. Any later code/content reuse still follows the separate review in
[THIRD_PARTY.md](../THIRD_PARTY.md). Preserve the existing physical controls and
host boundary when turning research into an implementation.
