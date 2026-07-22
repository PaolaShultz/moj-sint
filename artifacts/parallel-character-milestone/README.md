# Parallel character listening gate

Nine loudness-matched dry/moderate/strong files cover MIDI notes 36, 60, and 84. The user listening verdict is open.

Listen to each note in dry, moderate, strong order without additional normalization. Reject the branch if it reads as minor EQ or generic distortion; do not expand the matrix before that decision.

`character-manifest.tsv` records level, DC, pitch, and hashes; `harmonic-distribution.tsv` records the first 12 partials; `alias-comparison.tsv` records method selection; `intermodulation.tsv` compares per-voice and post-mix placement.

The selected production method is `direct`. Workstation timing is scalar development evidence only, not Raspberry Pi evidence.
