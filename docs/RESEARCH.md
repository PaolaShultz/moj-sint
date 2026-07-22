# Research Notes and Source Register

Sources are primary papers, project documentation, or official tool documents.
Equations and described topologies may guide an independent implementation;
third-party source code, presets, and samples must not be copied without a
separate license review.

## Oscillators and nonlinear DSP

- Vesa Välimäki, Jussi Pekonen, and Juhan Nam, “Perceptually Informed
  Synthesis of Bandlimited Classical Waveforms Using Integrated Polynomial
  Interpolation,” *JASA* 131(1), 2012.
  [Author manuscript](https://mac.kaist.ac.kr/pubs/ValimakiPeknenNam-jasa2012.pdf).
  Establishes and perceptually evaluates PolyBLEP variants; useful for the first
  bandlimited discontinuous oscillator. The PDF carries ASA redistribution
  terms, so use the mathematics and citation, not copied prose/figures/code.
- Günter Geiger, “Table Lookup Oscillators Using Generic Integrated
  Wavetables,” DAFx-06, 2006.
  [DAFx record](https://www.dafx.de/paper-archive/details/ocJiGEUzdj2dyPlyCGROcw)
  and [paper PDF](https://www.dafx.de/paper-archive/2006/papers/p_169.pdf).
  Useful comparison point for wavetable and differentiated-integral approaches.
- Joseph Timoney, Victor Lazzarini, Jari Kleimola, Jussi Pekonen, and Vesa
  Välimäki, “Virtual Analog Oscillator Hard Synchronisation,” DAFx-12, 2012.
  [Paper](https://dafx.de/paper-archive/2012/papers/dafx12_submission_37.pdf).
  Compares phase-reset and efficient Fourier/filter formulations.
- Pier Paolo La Pastina and Stefano D'Angelo, “A General Antialiasing Method
  for Sine Hard Sync,” DAFx-22, 2022.
  [Paper](https://dafx.de/paper-archive/2022/papers/DAFx20in22_paper_3.pdf).
  A newer FIR-based hard-sync method and useful cost/aliasing benchmark.
- John M. Chowning, “The Synthesis of Complex Audio Spectra by Means of
  Frequency Modulation,” *JAES* 21(7), 1973.
  [AES record](https://secure.aes.org/forum/pubs/journal/?elib=1954).
  Foundational FM spectral/control model. Publication copyright applies.
- Fabián Esqueda, Henri Pöntynen, Vesa Välimäki, and Julian D. Parker,
  “Virtual Analog Buchla 259 Wavefolder,” DAFx-17, 2017.
  [Paper](https://dafx.de/paper-archive/2017/papers/DAFx17_paper_82.pdf).
  Concrete antialiased multi-stage wavefolder reference using BLAMP and
  oversampling.
- Julian D. Parker, Vadim Zavalishin, and Efflam Le Bivic, “Reducing the
  Aliasing of Nonlinear Waveshaping Using Continuous-Time Convolution,”
  DAFx-16, 2016.
  [Archive record](https://www.dafx.de/paper-archive/details/vem_XXF5qBbfiWOH2RVVAA).
  Introduces the continuous-time/antiderivative line for nonlinear antialiasing.
- Stefan Bilbao, Fabián Esqueda, Julian D. Parker, and Vesa Välimäki,
  “Antiderivative Antialiasing for Memoryless Nonlinearities,” *IEEE Signal
  Processing Letters* 24(7), 2017, DOI 10.1109/LSP.2017.2675541.
  [University record and manuscript](https://www.research.ed.ac.uk/en/publications/antiderivative-antialiasing-for-memoryless-nonlinearities/).
  Generalizes ADAA orders; validate numerical corner cases independently.
- Martin Holters, “Antiderivative Antialiasing for Stateful Systems,” *Applied
  Sciences* 10(1), 2020, DOI 10.3390/app10010020.
  [Open-access article](https://www.mdpi.com/2076-3417/10/1/20).
  Relevant when nonlinearities enter feedback/stateful structures; article is
  CC BY 4.0, but no source implementation is imported here.

Future source passes should separately cover phase distortion, coupled/chaotic
oscillators, physical models, feedback-delay networks, nonlinear filters, and
denormal/DC handling before selecting an unfamiliar-sound architecture.

### 2026-07-22 oscillator comparison implementation

The first oscillator milestone independently implemented two scalar method
families from the registered papers; no third-party DSP source, tables, prose,
presets, samples, or figures were copied.

- The PolyBLEP candidate applies the second-order residual obtained by
  integrating first-order linear interpolation at the saw and square
  discontinuities. This is the smallest polynomial method described by
  Välimäki, Pekonen, and Nam and uses no lookup table.
- The generic integrated-wavetable candidate stores immutable antiderivative
  tables for the same saw and square targets, linearly interpolates them, then
  restores the waveform through one-sample differentiation and phase-increment
  normalization. This follows Geiger's integration/differentiation method while
  using exact target antiderivatives to avoid adding table-construction error to
  the comparison.

Both candidates used the same phase convention, target morph, 48 kHz rate, 32
coherent periods, and representative note/shape matrix. The non-real-time
metric removes DC and fits one scalar gain before comparing with a finite
band-limited Fourier reference. Because the residual includes amplitude and
phase approximation error as well as alias products, it is named
`alias_error_db` and treated as a conservative alias/error measurement.

The recorded aggregate residual is -25.188 dB for PolyBLEP and -25.586 dB for
the integrated wavetable. Worst cases are effectively tied at -18.325 dB; the
predeclared 0.1 dB tie window therefore selects the lower aggregate integrated-
wavetable result. This is engineering evidence for the current route, not a
claim of perceptual superiority. Raw conditions and rows are in
`artifacts/oscillator-milestone/oscillator-comparison.tsv`.

The JASA author manuscript retains ASA redistribution/copyright terms, and the
DAFx paper retains its publication rights. Moj Sint uses only the described
mathematics and citations.

## Distinctive oscillator systems and routing survey

This 2026-07-22 pass asks what actually distinguished several influential
instruments and which ideas are useful for Moj Sint. “Legendary” is a cultural
and musical judgment, not a measurable engineering category. The durable
technical lesson is that identity usually came from a whole signal path,
modulation topology, control law, or useful imperfection rather than from one
isolated ideal oscillator.

### Primary and authoritative sources

- Moog Music, *Minimoog Model D Manual*, current official reissue manual.
  [PDF](https://api.moogmusic.com/sites/default/files/2018-01/Minimoog_Model_D_Manual.pdf).
  The documented path mixes three oscillators, noise, and external input before
  the filter; high mixer/output settings and feedback can overload that path.
  This supports treating oscillator beating, mixer saturation, ladder behavior,
  and feedback as a system rather than attributing the sound to a raw saw wave.
  The manufacturer manual is copyrighted; use facts and independently developed
  DSP, not copied prose, figures, circuits, or presets.
- Sequential, *Prophet-5 User's Guide*, version 1.3, 2021.
  [PDF](https://sequential.com/wp-content/uploads/2021/02/Prophet-5-Users-Guide-1.3.pdf).
  Its Poly-Mod sources are the filter envelope and Oscillator B; destinations
  are Oscillator A frequency, Oscillator A pulse width, and filter cutoff. The
  manual explicitly connects this topology to many original classic sounds and
  notes that the modulator waveshape changes the modulation character. This is
  strong evidence for routing order and source shape as first-class timbre.
- Buchla, *259e Twisted Waveform Generator* documentation.
  [Official documentation](https://buchlausa.github.io/buchla_doc/docs/200e/259e-md).
  It pairs principal and modulation oscillators and routes the latter to pitch,
  wavetable morph, or warp. The principal oscillator's sine drives selectable
  wavetables whose morph and drive level become timbre controls. For the older
  analog 259 wavefolder, Esqueda, Pöntynen, Välimäki, and Parker's DAFx-17 model
  remains the technical reference already registered above; the publication is
  CC BY and reports five parallel folding stages plus a direct path and an
  antialiasing treatment using BLAMP with eight-times oversampling.
- Yamaha, *DX7 Operating Manual*, official scanned edition.
  [PDF](https://usa.yamaha.com/files/download/other_assets/9/333979/DX7E1.pdf).
  Read alongside John Chowning's 1973 paper registered above. The important
  design lesson is not “six expensive oscillators”: simple operators become a
  broad instrument through frequency ratios, level envelopes, 32 connection
  algorithms, multiple carriers, and feedback. Moj Sint should investigate
  smaller purpose-designed graphs before assuming a six-operator clone.
- Masanori Ishibashi / Casio Computer Co., US patent 4,658,691, *Electronic
  musical instrument*, priority 1982, published 1987.
  [Patent record](https://patents.google.com/patent/US4658691).
  It describes changing the rate of a waveform-memory address within each cycle
  so a timbre control changes the spectrum. This phase/address-warp principle is
  computationally attractive, but abrupt phase-law corners alias and require
  the same measurement discipline as hard sync and other discontinuities. The
  patent has ceased according to the record; patent text/figures still are not
  project prose or assets.
- Wolfgang Palm, first-person PPG development account, “Wavecomputer.”
  [Author account](https://palm.seib.info/story/c7.html).
  Palm describes placing analyzed/synthesized spectra at wavetable positions
  and playing them with a digital oscillator; the resulting spectral traversal
  produced the characteristic bright “jingle” rather than an analog filter
  imitation. This supports motion through related spectra as an oscillator
  design, not merely a large static waveform menu.
- Roland, *JP-8000* official product document.
  [PDF](https://cdn.roland.com/assets/media/pdf/JP-8000.pdf). It describes the
  Super Saw as seven simultaneous saws with DETUNE and MIX controls, alongside
  Triangle Mod, Feedback Oscillator, sync, ring modulation, and cross
  modulation. Adam Szabo, *How to Emulate the Super Saw*, KTH Royal Institute
  of Technology bachelor's thesis, 2010,
  [PDF](https://www.adamszabo.com/internet/adam_szabo_how_to_emulate_the_super_saw.pdf),
  measured phase, detune, mix, high-pass, and aliasing details. The thesis is a
  reverse-engineering source, not permission to copy code, samples, prose, or
  exact proprietary presets.
- Victor Lazzarini and Joseph Timoney, “Higher-Order Frequency Modulation
  Synthesis,” 2023. [Preprint](https://arxiv.org/abs/2305.07909). It develops
  higher-order direct-FM arrangements through an equivalent PM formulation and
  treats feedback FM. The equations may guide an independent implementation;
  do not copy the accompanying reference source without separate license review.
- David Morrison DeFilippo, *Coupled Oscillator Systems*, UC San Diego doctoral
  dissertation, 2023.
  [Repository record and PDF](https://escholarship.org/uc/item/6bc2r6n5).
  It studies seven velocity-coupled systems containing three to nine
  oscillators and analyzes their nonlinear/nonstationary behavior. It supports
  coupled networks as a real synthesis family, while also showing that topology
  needs analysis rather than arbitrary connection for its own sake.
- Antti Huovilainen, “Non-Linear Digital Implementation of the Moog Ladder
  Filter,” DAFx-04, 2004.
  [Paper](https://dafx.de/paper-archive/2004/P_061.PDF). It derives a real-time
  cascade of nonlinear first-order sections from circuit equations. This is a
  useful procedural-model cost reference: component-inspired behavior can be
  practical, but nonlinear stages, tuning correction, feedback, and
  oversampling are materially more expensive than one table lookup.

### What “good” and “bad” sound mean here

No scalar metric establishes good sound. For Moj Sint, automated evidence must
instead reject unintended failure and describe intentional character:

- retain a stable perceived pitch or document when a topology deliberately
  gives it up;
- produce useful spectral motion over most macro travel rather than a tiny
  sweet spot surrounded by sameness or noise;
- control DC, peak, RMS, low-note energy, high-note spectral collapse, and
  discontinuities under static settings and rapid movement;
- distinguish intended inharmonicity, beating, roughness, aliasing, or chaos
  from accidental numerical instability and sample-rate-dependent garbage;
- preserve deterministic replay unless seeded variation is explicitly part of
  the preset; and
- pass human listening across notes, velocities, durations, phrases, and mix
  contexts. Measurements can reject defects and compare changes, but cannot
  declare an oscillator musical.

Historically characteristic artifacts must not be removed automatically. For
example, a measured legacy ensemble may include aliasing or unusual filtering
that listeners associate with its identity. Moj Sint may retain a controlled
artifact only after A/B renders show that it is intentional, bounded, and more
valuable than the cleaner alternative. “Analog drift,” “warmth,” and “fatness”
are not specifications until converted into reproducible behaviors.

### Cost classes before Raspberry Pi measurement

These are relative algorithmic expectations, not Raspberry Pi performance or
polyphony claims:

| Family | Expected scalar cost | Main risk |
| --- | --- | --- |
| Table/BLEP/DPW oscillator | Low, bounded work per sample | Limited topology or residual aliasing |
| Phase distortion or 2–3 PM operators | Low to moderate, roughly per operator/edge | Wide sidebands, DC/pitch errors, aliasing |
| Shared-phase 3-tap bank | Low if taps share phase/table state | Linear cancellation can make controls inert |
| Independently detuned ensemble | Roughly proportional to oscillator count | CPU/state multiplication and level growth |
| Wavefold/drive/ring/nonlinear modulation | Moderate before antialiasing, potentially high after it | Frequency expansion often needs ADAA/BLAMP or oversampling |
| Circuit-inspired nonlinear feedback | Moderate to high and topology-dependent | Solver/oversampling cost and stability |
| Coupled ODE/chaotic network | Highest uncertainty; scales with nodes, edges, and integration substeps | Instability, pitch loss, sample-rate dependence |
| Additive/resynthesis | Proportional to active partial count | Low-note cost and macro preparation |

The next monophonic experiments should measure workstation instruction/time
cost only as development evidence. Real Raspberry Pi callback timing and
headroom decide whether and how the chosen architecture expands to polyphony.

### Moj Sint experiment families

The user's intentionally speculative routing examples are valid research
directions when expressed as controlled graphs:

1. **Nonlinear modulator preprocessing.** Compare
   `modulator -> phase offset -> drive/fold -> DC block -> PM carrier` against
   `modulator -> drive/fold -> fractional delay/all-pass -> DC block -> PM
   carrier`. Offsetting oscillator phase and phase-shifting an already
   nonlinear rich waveform are different operations; nonlinearity and phase
   manipulation generally do not commute. Sweep drive, offset/delay, ratio,
   and index while matching RMS.
2. **Three-phase harmonic selector.** Derive 0°, 120°, and 240° taps from one
   pitch-locked phase source, optionally inject low harmonics into individual
   taps, apply different bounded nonlinearities, then weight and sum them.
   Equal linear taps cancel the fundamental and all harmonics not divisible by
   three; asymmetry, tap-specific processing, and macro-controlled weights turn
   that cancellation into a predictable spectral tool. This is cheaper and
   more coherent than assuming three independent VCOs, while independent drift
   remains a later variant.
3. **Small directed operator graph.** Use two or three pitch-related nodes with
   PM/FM, AM/ring, sync, or feedback edges. Compare edge ordering explicitly:
   source shaping before modulation, destination shaping after modulation, and
   one bounded feedback edge. Prefer PM semantics for nested graphs unless a
   direct-FM formulation proves pitch-stable.
4. **Complex-oscillator path.** Principal oscillator plus modulation oscillator,
   with separate modulation of pitch and nonlinear timbre. Compare parallel
   dry/folded paths against serial folding; do not start with a component-exact
   Buchla clone.
5. **Spectral traversal.** Interpolate through a small authored family of
   related spectra or phase laws, PPG/Casio-inspired but Moj Sint-owned. Test
   whether `EVOLVE` or `SHAPE` can traverse a coherent identity without turning
   into an arbitrary waveform browser.
6. **Controlled ensemble.** Compare shared-phase offsets, deterministic detune,
   seeded phase variation, and a small three-voice cluster before any seven-
   oscillator design. Measure whether thickness comes from beating, phase,
   mix law, or filtering rather than merely adding oscillators.
7. **Coupled nonlinear trio.** Only after the bounded graphs above, prototype a
   three-node velocity- or phase-coupled system with explicit energy limiting.
   Map stable, quasi-periodic, and unstable regions before exposing a macro.

Every family needs the same baseline, loudness-matched A/B renders, alias/error
and harmonic measurements, DC/peak/RMS/finiteness/determinism checks, rapid
parameter sweeps, render-path allocation tests, and a cost report. Reject a
graph that is merely complicated but not controllably different.

### 2026-07-22 shared-phase harmonic-selector experiment

The first oscillator-system experiment selected family 2, the shared
0/120/240-degree bank. Nonlinear modulator preprocessing was deferred because a
fair first pass also requires a modulation-phase API, DC handling, and a
nonlinear antialiasing decision. A directed PM/AM/feedback graph was deferred
because its ratios, edge order, feedback bounds, and sideband behavior create a
larger experiment than the shared-phase invariant.

The implementation is independently derived from the elementary phase
identities. One per-voice recursive sine/cosine state produces all three taps
with fixed coefficients. Equal linear taps cancel. Cubing each tap, summing,
and multiplying by -4/3 isolates a unit third harmonic because the linear terms
cancel and all three `sin(3x)` terms align. No third-party source, table, preset,
sample, prose, or figure was imported. The existing source authorship,
publication URLs, and licensing notes above remain the provenance boundary.

`EDGE` traverses from the zero-degree fundamental tap to the cubic three-tap
third harmonic. `COUPLE` traverses from the existing `SHAPE`/`COLOR` path to
that selector. Both are smoothed over 10 ms and use bounded algebraic level
compensation. The `COUPLE=0` baseline is intentionally independent of `EDGE`.
State stays per voice, although the evidence phase renders exactly one voice.
The third-harmonic route stays full through 40% of sample rate, tapers to zero
before 48%, and then returns to the fundamental tap. The weight is prepared at
note frequency changes, not calculated with transcendental work per sample.

The experiment generated 27 loudness-matched renders covering notes 36/60/84
and all low/middle/high `EDGE`/`COUPLE` combinations. Peak was
0.107426–0.142435,
active-window RMS was 0.079778–0.080013, and maximum absolute DC was
0.000000297. The pure third-harmonic endpoints deliberately remove the fitted
fundamental and report pitch retention as false; the other 24 conditions retain
it under the declared 30 dB rule. Conservative non-harmonic residual ranges
from -100.118 dB to -27.053 dB. Tests cover deterministic reset, exact
third-harmonic selection, rapid movement, finite/bounded output, and allocation-
free oscillator and engine render paths.

The separate high-note alias matrix covers MIDI 84/96/108/114/117/120/127.
The cubic third remains full through note 114, is half weighted at note 117,
and has fallen back to the fundamental before its target crosses Nyquist at
notes 120 and 127. Every row is finite and its measured non-harmonic residual
is between -85.907 dB and -63.405 dB. This is alias/error evidence for the
implemented guard, not a general claim that every later nonlinear route is
alias-free.

The release-build workstation micro-timing is stored separately from the
deterministic manifest because it is volatile. It describes only the isolated
primitive and is not Raspberry Pi or callback evidence. Automated measurements
established controlled change, not musical usefulness.

The user then listened to all generated conditions and rejected the experiment:
the outputs sounded like largely undifferentiated pure-sine material and offered
no useful distinctive identity. That is consistent with the graph itself,
which only crossfades a sine fundamental and a sine third harmonic. The
generated artifact directory was removed after review. The deterministic lab
command remains because tests use temporary output, but the `EDGE`/`COUPLE`
mapping is not musically accepted and must not be promoted into a factory
preset. The next research phase should preserve a base sound while comparing
parallel nonlinear, excited-harmonic, subharmonic, or feedback layers rather
than treating sparse sine partial selection as sufficient identity.

### 2026-07-22 parallel character-layer experiment

The next phase compared three small dry-plus-return graphs before changing the
engine:

1. a full-band symmetric soft-clipped copy, which was smallest but exposed all
   existing high partials to nonlinear aliasing and risked generic distortion;
2. a band-limited symmetric generated-residual return, which preserved the dry
   path and was selected as the first implementation; and
3. a band-pass biased/asymmetric exciter, which could add even and odd products
   but also required a bias, two cutoff policies, and stronger DC control.

Subharmonic reinforcement was separately compared as a later family. A
per-voice oscillator at half frequency has the clearest octave and future chord
semantics. Period division adds waveform/onset policy, while post-mix tracking
becomes ambiguous for chords. None was combined with this experiment. Placement
was selected per voice before mixing so state remains pitch-specific and the
nonlinearity cannot create inter-voice products. A measured 600/900 Hz two-tone
probe found -134.381 dB combined intermodulation for separated direct branches
and -15.463 dB when the same character process was applied post-mix. This is a
controlled placement comparison, not a polyphony or audibility claim.

The implemented branch keeps the existing `SHAPE`/`COLOR` sample as an
untouched dry anchor. Its copy passes through four cascaded one-pole low-pass
sections at 6 kHz, a symmetric cubic soft clip with constant continuation,
linear-component subtraction, a 10 Hz DC blocker, and two cascaded note-tracked
high-pass sections. The latter are prepared at 2.5 times note frequency and
capped at 6 kHz, preventing the generated return from cancelling the dry
fundamental. The return polarity was selected to add rather than cancel upper
content and its gain is explicitly bounded. `EDGE` supplies the candidate
drive/intensity route from 1x to 2x; `COUPLE` supplies the candidate return
route. Both retain 10 ms smoothing, and `COUPLE=0` is sample-identical for all
`EDGE` values. These are research mappings, not accepted stable factory
behavior.

Early measured revisions were rejected before handoff. A high return without
the note-tracked filter partially cancelled the fundamental, while the first
conservative return moved the third harmonic by less than 1 dB and looked like
minor level/EQ change. The retained revision requires a 440 Hz sine probe to
keep fundamental amplitude above 0.7 while adding a third-harmonic amplitude
above 0.05. Over the real saw/square base, the strong listening conditions lift
partials 4-12 by roughly 1-1.5 dB on MIDI 36/60; MIDI 84 changes less and is a
known limitation. The 12-partial rows are recorded rather than calling that
change musically useful.

Direct, first-order ADAA, and a two-times interpolated research path were
compared against an independently rendered eight-times direct reference. A
zero-phase 127-tap windowed-sinc low-pass removes reference content above 90%
of target Nyquist; only the generated path receives the half-sample alignment
used by ADAA and the two-times variant. The predeclared selection requires at
most -60 dB residual at moderate drive, -50 dB at strong drive, and no more than
3 dB regression from the best worst case. Reducing the drive ceiling from 8x
to 2x was necessary before any method passed. Direct evaluation then measured
-73.700 dB worst-case moderate and -59.121 dB worst-case strong and was selected
as the cheapest passing method. ADAA measured -53.093/-49.502 dB and the
two-times path -64.365/-61.442 dB. Although the two-times path has the lowest
strong residual, direct stays within the declared window and passes both
absolute limits. This result applies only to this filtered 1x-2x graph; it does
not establish that direct evaluation is generally preferable for nonlinear DSP.
BLAMP was not implemented because the graph has no known fold/reset
discontinuity to correct.

The nine loudness-matched 48 kHz stereo listening files are `note036_dry.wav`,
`note036_moderate.wav`, `note036_strong.wav`, and the corresponding note 60 and
84 triplets under `artifacts/parallel-character-milestone/`. Active-window RMS
is 0.079769-0.079980, peak is 0.139552-0.143008, maximum absolute DC is
0.000000196, and every row retains the fitted fundamental. The manifest, 12-
partial distribution, alias comparison, intermodulation table, and volatile
workstation timing accompany the files. The user listening verdict remains
open; automated checks do not accept `EDGE`, `COUPLE`, or the graph as a factory
sound.

The independently implemented ADAA comparison uses the mathematics described
by Stefan Bilbao, Fabián Esqueda, Julian D. Parker, and Vesa Välimäki,
“Antiderivative Antialiasing for Memoryless Nonlinearities,” *IEEE Signal
Processing Letters* 24(7), 2017, DOI 10.1109/LSP.2017.2675541,
[author manuscript](https://www.research.ed.ac.uk/files/34115216/bilbao_pdf.pdf).
No third-party source, presets, samples, prose, or figures were copied.
Workstation cost is scalar x86_64 development evidence only. It is not native
Raspberry Pi callback, latency, safe-polyphony, or sound-quality evidence.

## Real-time I/O and platform

- JACK project, [API overview](https://jackaudio.org/api/) and
  [real-time callback requirement](https://jackaudio.org/api/group__NonCallbackAPI.html).
  These define the callback constraints used in `HOST_CONTRACT.md`.
- ALSA project, [Sequencer interface](https://www.alsa-project.org/alsa-doc/alsa-lib/seq.html)
  and [event definitions](https://www.alsa-project.org/alsa-doc/alsa-lib/group___seq_events.html).
  These define clients, ports, subscriptions, timestamped MIDI-like events, and
  event storage caveats.
- Rust project, [rustup cross-compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
  and [platform support](https://doc.rust-lang.org/rustc/platform-support.html).
  Target installation is not a linker, sysroot, native run, or timing test.
- Raspberry Pi Ltd, [Raspberry Pi 5 product brief](https://datasheets.raspberrypi.com/rpi5/raspberry-pi-5-product-brief.pdf)
  and [audio-option whitepaper](https://pip-assets.raspberrypi.com/categories/1259-audio-camera-and-display/documents/RP-008124-WP-1-Choosing%20an%20Audio%20option.pdf).
  Hardware facts and I/O choices only; neither supports latency claims.

## Rust tools and integration candidates

- `jack` 0.13.5 (MIT, RustAudio): maintained JACK wrapper with default dynamic
  loading. Evaluate against local FFI; do not add until the live-host milestone.
- `alsa` 0.12.0 (MIT OR Apache-2.0): thin ALSA wrappers with Sequencer support.
  It requires ALSA development metadata; evaluate during live-host work.
- `assert_no_alloc` 1.1.2 (BSD-1-Clause): added as a dev dependency to enforce
  the block-render no-allocation test. It uses a test global allocator and is
  not shipped in the normal binary.
- `cargo-audit` (RustSec) checks `Cargo.lock` against the advisory database:
  <https://rust.dev/tools/cargo-audit>.
- `cargo-deny` checks advisories, licenses, duplicate dependencies, and sources:
  <https://embarkstudios.github.io/cargo-deny/>. Its own documentation warns
  that metadata-based license checks do not replace human review.
- `hyperfine` is useful once there is a meaningful benchmark command, not for
  the reference sine. `perf`, `taskset`, `chrt`, and `gdb` are already present.
  Criterion is deferred because callback distributions and Pi-native evidence
  matter more than workstation microbenchmarks at this stage.

No installed Codex skill or MCP is specialized for Rust real-time DSP,
Raspberry Pi audio measurement, synth-source curation, or listening tests. The
general brainstorming, planning, TDD, and verification skills were useful;
creating new project skills before these workflows repeat would be speculative.
No OpenAI-specific MCP was used.

## Current dependency license snapshot

`cargo metadata` reports the normal shipped dependency graph as Apache-2.0,
MIT, or dual MIT/Apache-2.0. Platform-only transitive metadata also includes
Unicode-3.0, Apache-2.0 WITH LLVM-exception, and an LGPL alternative expression
for `r-efi`; the selected expression includes permissive alternatives. The Moj
Sint project license has not been selected, so the crate is marked
`publish = false`; choose and add a LICENSE file before distribution. Re-run
`cargo deny check` and manually inspect new DSP assets/code whenever
dependencies change.
