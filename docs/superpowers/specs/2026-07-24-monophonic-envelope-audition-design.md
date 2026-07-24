# Monophonic Composite Envelope Audition Design

Date: 2026-07-24

## Correction

The coherent-composite batch applied a master ADSR, but its 0.88 sustain was
held for almost the complete 10-12 second source render. The result therefore
sounded steady after the first fraction of a second. That met the mechanical
"one envelope after the sum" rule without providing the requested musical
envelope audition.

The replacement batch must answer one narrow listening question: how does the
promising exact composite respond to a very short, moderately longer, and
clearly slow shared attack?

## Approaches considered

1. **Controlled attack comparison — selected.** Keep one exact monophonic
   composite and vary only master attack time. This makes the audible
   difference attributable and does not invent another source.
2. **Individual original-layer envelope files — rejected.** These would
   audition isolated sounds, contrary to the requirement that the result be
   the complete combined sound.
3. **Rewrite chord and progression layers as new single-note sources —
   rejected.** This would alter the source mechanisms and reopen the failure
   mode where new sounds replaced the promising originals.

## Source boundary

Use only the existing `ThreeSingles` membership:

- `CrossSingle`;
- `SpectralSingle`; and
- `DualSingle`.

These are the three exact original single-note layers at their existing pitch.
Keep their DSP, score, seed, internal preparation, stereo construction, and
source-local safety fades unchanged. Sum them with the retained `0/2/5 ms`
fixed offsets and equal-power layer trim.

"Monophonic" means one musical pitch rather than a chord or progression. The
output remains two-channel stereo; it is not a mono fold-down.

No chord, progression, twelve-layer orientation reference, solo layer, mono
diagnostic, or old steady-envelope WAV belongs in the listening batch.

## Shared master-envelope profiles

Apply exactly one amplitude envelope to the complete stereo sum. All profiles
share:

- audition duration: `2400 ms`;
- decay: `220 ms`;
- sustain: `0.58`;
- release: `500 ms`;
- release start: `1900 ms`.

Only attack changes:

1. `6 ms` — very short;
2. `35 ms` — moderately longer;
3. `140 ms` — slow onset.

The 6 ms attack still covers the latest 5 ms layer onset. Every render begins
at zero and both stereo channels return to zero together on the final frame.
There is no 10-12 second sustain.

## Level and rejection policy

Use one shared fixed presentation gain for all three profiles, followed by the
existing static `-0.3 dBFS` ceiling. Do not normalize profiles individually.
Select the shared gain under these constraints:

- whole-file stereo RMS must not exceed `-10 dBFS`;
- hard-ceiling contact must not exceed `1%` of all samples.

After applying the shared gain, reject and omit any profile whose whole-file
stereo RMS is below `-14 dBFS`. Total RMS includes the complete 2.4-second
file, not an automatically cropped active window.

Also omit non-finite, noise-classified, excessive-DC, excessive-jump,
stereo/mono, tonal, or non-zero-tail failures. A rejected profile remains a
report row and never becomes a WAV.

No compressor, automatic limiter, soft saturator, maximizer, time-varying
make-up gain, or post-render normalization is allowed.

## Listening output

Replace all old artifacts with one ignored directory:

`artifacts/monophonic-envelope-composites/`

It may contain at most these three WAVs, in this order:

1. `01_monophonic_attack_006ms.wav`;
2. `02_monophonic_attack_035ms.wav`;
3. `03_monophonic_attack_140ms.wav`.

Reports must state the exact shared source membership, offsets, envelope
parameters, shared gain, total RMS, active RMS, peak, ceiling proportion,
tonal/noise result, stereo/mono evidence, tail result, and deterministic
hashes. The README must say that these are envelope positions of one sound,
not fundamentally different source mechanisms.

## Tests and verification

Add tests proving:

- every listening render uses only the three exact single-note layers;
- all three profiles have the declared parameters and 2.4-second duration;
- the shared master envelope is post-sum, allocation-free, finite, and
  stereo-common;
- the first and final frames are zero;
- every retained WAV meets the whole-file RMS floor;
- a deliberately low-total-RMS profile is reported but not written;
- no chord, progression, solo, diagnostic, reference, or rejected WAV appears;
- two independent generations are byte-identical except workstation timing.

Run the repository's complete format, all-target tests, Clippy, release,
audit, deny, AArch64, deterministic-render, and artifact-hygiene checks before
handoff.

Automated evidence can reject obvious defects. Human listening decides which
attack behavior, if any, is musically useful.
