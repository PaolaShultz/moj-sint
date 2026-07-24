# Coherent Hybrid Composite Envelope Design

Date: 2026-07-24

## Correction

The prior subset batch misinterpreted delayed launch. It preserved delays of
roughly one to four seconds between selected layers, so components entered as
separate audible events. The intended sound is one composite voice: all three
or four selected original hybrid layers start together, with only tiny fixed
offsets that change phase and summation.

The final workspace must contain no older generated experiment batch. Only the
new ignored `artifacts/coherent-hybrid-composites/` directory may remain.

## Source and timing boundary

Keep the seven approved three- and four-layer combinations and the exact
original hybrid source pool. Do not add or replace oscillators, mechanisms,
scores, seeds, fades, stereo paths, or internal source processing.

Use these fixed offsets in selection order:

- three-layer combinations: `0, 2, 5 ms`;
- four-layer combinations: `0, 4, 9, 15 ms`.
- twelve-layer orientation reference:
  `0, 1, 3, 4, 5, 7, 8, 9, 11, 12, 14, 15 ms`.

Offsets are relative to the common note/composite trigger. They do not inherit
the old player-process delays. Every layer is active before the master attack
finishes, so no component is presented as a separate sound.

## Shared master envelope

After applying the fixed layer offsets and summing the selected layers:

1. apply one equal-power pre-sum trim of `1 / sqrt(layer_count)` to every
   selected layer;
2. apply one shared master ADSR to the complete stereo sum;
3. apply one explicit fixed presentation gain for the complete candidate; and
4. apply the static `-0.3 dBFS` hard ceiling.

The master ADSR is identical for every candidate:

- attack: `25 ms`;
- decay: `180 ms`;
- sustain: `0.88`;
- release: `320 ms`.

The gate remains open until `320 ms` before the complete composite ends. The
master envelope starts at zero, covers every micro-delayed layer onset, and
returns both channels to zero together. Layer-local fades and score transitions
remain source-internal safety/details; they are not separate audible master
envelopes.

No compressor, automatic limiter, soft saturator, maximizer, time-varying
make-up gain, or post-render normalization is allowed. Presentation gains are
reported per composite because one shared three-layer gain cannot keep both
the single-note and mono-progression composites inside the full-rate
`-14` to `-10 dBFS` gate.

## Listening output

The new batch contains:

1. one twelve-layer orientation reference rebuilt with the specified
   `0–15 ms` fixed micro-delay spread and the same master envelope; and
2. the seven three- or four-layer complete composite candidates.

No isolated source, solo layer, long-delay reconstruction, mono diagnostic,
parameter sweep, or rejected WAV is presented.

The listening order remains:

1. reference;
2. all-family single-note composite;
3. all-family held-chord composite;
4. all-family stereo-progression composite;
5. all-family mono-progression composite;
6. Cross anchor;
7. Spectral anchor; and
8. Dual anchor.

“Single-note composite” means three complete source layers sounding together;
it is not an isolated source audition.

## Engineering gate

Retain the previous moderate-hot bounds:

- active RMS: `-14` to `-10 dBFS`;
- hard-ceiling contact: at most `1%` of active samples;
- static ceiling: `-0.3 dBFS`;
- absolute DC: at most `0.001`;
- maximum adjacent-sample jump: at most `1.5`;
- mono loss: at most `1.5 dB`;
- left/right correlation: greater than `-0.25`;
- scheduled tonal target pass fraction: at least two thirds per source
  segment;
- noise classification only when spectral flatness exceeds `0.50` and tonal
  coverage also fails; and
- eight-times post-sum nonlinear residual more negative than `-1 dB`.

Add tests proving that no candidate offset exceeds `15 ms`, all layers enter
before the `25 ms` master attack completes, the master envelope is applied
once after the sum, sampling remains allocation-free, and the final stereo
tail reaches zero together.

Automated tests reject obvious defects but cannot select musical value.

## Scope and retention

Keep the work isolated from `Engine`, presets, stable macros, JACK, ALSA,
SHR-DAW, production polyphony, and architecture-specific DSP.

Generated WAVs and reports remain ignored and disposable. Durable source,
tests, and concise handoff/research conclusions are tracked. At handoff,
`artifacts/coherent-hybrid-composites/` must be the only child of
`artifacts/`.
