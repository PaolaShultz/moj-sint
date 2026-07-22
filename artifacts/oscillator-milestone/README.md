# Oscillator milestone listening renders

These 27 deterministic stereo 32-bit float WAVs are the human-listening gate
for the first integrated-wavetable route. They use 48 kHz, velocity 0.8, a
1.25-second note, and the reference Moj Sint envelope.

The filename is the experiment condition:

```text
note060_shape050_color100.wav
        ^ MIDI 60
                ^ SHAPE 0.50
                         ^ COLOR 1.00
```

Notes 36, 60, and 84 each have the complete 0.00/0.50/1.00 Cartesian matrix
for both macros. Listen first within each note while changing one filename axis
at a time, then compare the same setting across notes.

- `SHAPE` moves from corrected saw toward corrected square. Midpoint level
  compensation is bounded and deliberately keeps the center from collapsing.
- `COLOR` moves from a note-tracked harmonic-dark path toward the direct bright
  oscillator.
- `listening-manifest.tsv` records peak, RMS, DC, and deterministic sample hash.
- `SHA256SUMS` records file-level hashes.
- `oscillator-comparison.tsv` contains the fair candidate measurement rows;
  `oscillator-comparison.md` summarizes the selection.

Automated sweeps prove that adjacent quarter-travel macro segments change the
waveform, remain finite/non-silent, stay smoothed, and respect the tested level
bound. They do not prove that either macro is musically useful. Human listening
acceptance remains required, and Raspberry Pi performance remains unmeasured.
