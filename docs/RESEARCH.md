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
