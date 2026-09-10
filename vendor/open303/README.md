# Open303 core provenance and local repairs

Author: Robin Schmidt, MIT copyright 2009. Imported from
[Open303 revision 313bf0d9ade7c1dcb6b3a74f5ea1780a29d70074](https://github.com/RobinSchmidt/Open303/tree/313bf0d9ade7c1dcb6b3a74f5ea1780a29d70074),
revision dated 2024-03-29, reviewed/imported 2026-09-09.
Original location: `Source/DSPCode/`. The separate license review is in
[the source analysis](../../docs/OPEN303_ANALYSIS.md). Retain `LICENSE-MIT`
and `LICENSE-OOURA` with redistribution. Ooura's `fft4g.c` is separately
permitted and byte-identical to the author's original package; do not
relabel it as Robin Schmidt's MIT work. No VST/JUCE/JC-303 wrapper, SDK,
factory program, sample, artwork, or sequencer content is imported.

`UPSTREAM.sha256` records the original 40-file DSP directory. The six
AcidPattern, AcidSequencer, and MidiNoteEvent header/source files are excluded.
`LOCAL.sha256` is the exact 34-file reviewed import manifest, including local
repairs. Verify from this directory with `sha256sum -c LOCAL.sha256`.
All C++ and header text was normalized from Latin-1/CRLF to UTF-8/LF;
`fft4g.c` was retained byte-for-byte. Compare each remaining file with its
pinned upstream counterpart when updating. Do not regenerate manifests to
accept an unexplained change.

## Local patch ledger

- `GlobalDefinitions.h`, `rosic_FunctionTemplates.h`: required standard
  headers, fixed-width UINT32, `memcpy` exponent extraction with signed bias
  subtraction. `rosic_NumberManipulations.h` and `rosic_RealFunctions.h`:
  remove obsolete x87 assembly and conflicting custom log2; portable sin/cos.
- `rosic_MipMappedWaveTable.{h,cpp}`: constructor writes only the actual 2048
  prototype elements; clamp table index at the exclusive upper bound;
  preparation scratch is local, permitting independent concurrent instances.
- `rosic_BlendOscillator.cpp`: initialize the reported blend to 0.5.
- `rosic_FourierTransformerRadix2.{h,cpp}`: prevent copying the owning FFT;
  real forward/inverse double-buffer overloads call rdft directly, preserving
  arithmetic without treating double arrays as Complex objects. Other legacy
  Complex convenience APIs are unexposed and outside the candidate contract.
- `rosic_Open303.{h,cpp}`: remove sequencer/list dependencies and replace the
  note stack with 128 unique fixed entries, last-note priority, bounded MIDI
  validation, stale-off handling, and tuned/accented held-note return. Add
  release-all, complete silent reset, and a 64-sample quiet-tail idle return.
  Initialize blend/volume/normalizers/slide/attack consistently, update the
  sample-rate member, forbid copying, and remove obsolete sequencer state.
  Tuning, accent, and normal decay update held notes. Remove ineffective
  normalizer calculations, retaining their original effective unity result.
- `rosic_AnalogEnvelope.cpp`, `rosic_DecayEnvelope.h`: reset complete
  envelope state without reconstructing or reallocating the engine.
- `GlobalFunctions.h`: remove the unused random generator and its
  Numerical Recipes-derived helper comment along with the sequencer path.

The adapter in `../../native/open303` and Rust/tests/lab are project-authored.
They restrict the exposed API, prepare a fixed filter mode/sample rate,
validate/cache controls, count output ceiling hits, and silence nonfinite
faults until reset. The ceiling is not part of the inherited filter model.
No oscillator tables, filter coefficients, distortion curve, or oversampling
algorithm was redesigned. Repaired initialization and note/reset semantics
mean this candidate is not bit-identical to all upstream behavior.

The broader public C++ API is not supported for direct production use. Only
the restricted owning Rust/C boundary is covered by the candidate tests.

Integration repair: `setAccentDecay` now updates an active accented main
envelope without retriggering. The C boundary validates/caches four native
articulation setters used by the live adapter. This extends the reviewed
interface without changing the oscillator/filter algorithms.

Live articulation repair: an explicit retrigger option on `noteOn` refreshes
both native contours for overlapping/repeated presses without resetting the
pitch slew or sounding oscillator/filter state. Amp attack starts at its
current level. Held-note return and the ordinary candidate/native API retain
their non-retriggering legato behavior. The C boundary exposes a separate
validated live-note function; no schema, controls, or DSP algorithms change.
