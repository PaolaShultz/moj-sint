# Original Hybrid Subset Mix Design

Date: 2026-07-24

## Goal

Return to the first strongly positive Moj Sint listening result: the accidental
simultaneous playback of the twelve original hybrid renders. Build a small
listening gate from combinations of three or four of those exact renders.

The experiment must not replace the successful ingredients with new
oscillators, mechanisms, envelopes, events, effects, or compact machines. It
must present a moderately hot signal with sparse hard-limited crests, not a
quiet conservative mix and not a continuously clipped noise block.

## Fixed source pool

The only eligible source layers are the twelve original deterministic hybrid
renders:

| Family | Eligible conditions |
| --- | --- |
| Cross-Coupled Machine | single, held chord, stereo progression, mono progression |
| Spectral Shadow | single, held chord, stereo progression, mono progression |
| Dual Resonant Body | single, held chord, stereo progression, mono progression |

Every selected layer retains its existing DSP topology, score, notes, seed,
phase/state preparation, fades, channel construction, and internal level.
The existing reconstructed raw hashes remain regression evidence that the
source pool has not changed.

The experiment may only:

1. select three or four layers;
2. preserve their original relative delayed-launch offsets, rebased so the
   first selected layer starts at sample zero;
3. sum them linearly; and
4. apply the declared presentation gain and static hard ceiling described
   below.

There is no compressor, automatic limiter, soft saturator, maximizer, dynamic
make-up gain, filter, DC coloration, new envelope, pitch change, or nonlinear
cross-coupling between layers.

## Candidate matrix

Render these seven candidate combinations:

1. `three-singles`: all three families' single-note renders.
2. `three-chords`: all three families' held-chord renders.
3. `three-stereo-progressions`: all three families' stereo progressions.
4. `three-mono-progressions`: all three families' mono progressions.
5. `cross-anchor`: Cross single, Spectral chord, Dual stereo progression, and
   Cross mono progression.
6. `spectral-anchor`: Spectral single, Dual chord, Cross stereo progression,
   and Spectral mono progression.
7. `dual-anchor`: Dual single, Cross chord, Spectral stereo progression, and
   Dual mono progression.

The full twelve-layer delayed-launch reconstruction is the first listening
file and the only reference. It is not a candidate and does not satisfy the
three-or-four-layer rule.

No random combinations, parameter variants, synchronized duplicates, mono
diagnostic WAVs, or newly designed sounds may be added to make the batch look
larger. A candidate that fails the engineering gate is omitted rather than
replaced with filler.

## Gain and crest policy

The previous conservative presentation around or below -20 dBFS was too quiet.
The later compact batch overcorrected, with prolonged heavy clipping and
noise-like flattening. This experiment uses moderate fixed gain into a static
hard ceiling:

- active RMS acceptance band: -14 to -10 dBFS;
- written sample ceiling: -0.3 dBFS;
- maximum hard-ceiling proportion: 1.0% of active samples;
- no compression or time-varying gain; and
- no per-file loudness normalization.

Three-layer candidates share one declared fixed linear gain. Four-layer
candidates share another. Before final generation, an engineering-only gain
sweep selects the largest shared gain in each group that keeps every retained
candidate at or below the 1.0% ceiling proportion. A candidate must also enter
the active-RMS band; otherwise it is rejected rather than granted a private
gain.

For these rules, the active region begins with the first nonzero frame after
the first selected layer starts and ends with the final nonzero release frame
of the last selected layer. RMS and ceiling proportion use both channels over
that region. dBFS RMS is `20 * log10(rms)`.

The twelve-layer reference has its own declared orientation gain because it is
outside both candidate groups. Its ceiling proportion must also remain at or
below 1.0%.

The hard ceiling is allowed to shave occasional crests and contribute a small
amount of audible edge. It must not become the main sound generator. Hitting
the ceiling for long blocks, visibly flattening the waveform body, or reducing
the output to an almost constant full-scale level is an automatic rejection.

## Noise and defect rejection

Automated measurements cannot decide whether a sound is musical, but they can
prevent obvious failures from reaching the user. A candidate is omitted when
any of these occur:

- non-finite samples, excessive DC, unintended silence, or failure to return
  to zero;
- more than 1.0% of active samples at the hard ceiling;
- active RMS outside -14 to -10 dBFS;
- loss of the scheduled tonal inventory: the known single, chord, or
  progression targets no longer remain measurable in their active segments;
- noise-like spectral flattening combined with failed tonal projections;
- catastrophic high-rate residual, discontinuity, or sample-rate-dependent
  instability beyond the unchanged source limitations;
- destructive stereo polarity or material mono-fold cancellation; or
- nondeterministic output or changed original-layer reconstruction hashes.

Spectral flatness alone does not reject a candidate because a useful hybrid
may contain noise or dense inharmonic energy. The noise rejection requires
both broadband flattening and loss of the expected tonal anchors. Severe
residuals already documented for an unchanged original source remain visible
in the report; they are not silently reclassified as clean.

The deterministic rejection thresholds are:

- absolute whole-file DC no greater than `0.001`;
- maximum adjacent-sample jump no greater than `1.5`;
- mono-fold RMS loss no greater than `1.5 dB` and left/right correlation
  greater than `-0.25`;
- fitted eight-times-rate residual more negative than `-1.0 dB` for the new
  post-sum hard-ceiling path; and
- for every original score segment, at least two thirds of the scheduled
  fundamental targets remain within `36 dB` of that segment's strongest
  expected target.

A result is classified as noise-like only when its magnitude-spectrum
geometric-mean/arithmetic-mean flatness exceeds `0.50` and it also fails the
two-thirds tonal-target rule. This conjunctive rule prevents broadband texture
alone from rejecting a pitched hybrid.

The generator writes a concise rejection table showing which condition removed
each failed candidate. Failed WAVs are not copied into the final listening
directory.

## Implementation boundary

This remains an isolated offline research experiment. Reuse the existing
original-layer reconstruction and hybrid render paths. Do not modify
`Engine`, presets, the thirteen stable macro roles, JACK, ALSA, SHR-DAW,
production polyphony, or architecture-specific DSP.

Construction and offline analysis may allocate. The per-sample subset mixer
and hard-ceiling path must be deterministic, finite, bounded, and
allocation-free. No avoidable per-sample transcendental setup is permitted.

All render-path changes are developed test-first. Tests lock:

- the exact twelve-layer source inventory and hashes;
- every candidate's exact three- or four-layer membership;
- rebased preservation of relative launch offsets;
- shared three-layer and four-layer gain policies;
- the -0.3 dBFS ceiling and 1.0% maximum ceiling proportion;
- finite, deterministic, allocation-free sampling;
- active RMS, tonal projection, noise-screen, stereo/mono, DC, and
  discontinuity rejection behavior; and
- exclusion of failed candidates from the listening directory.

## Disposable output and listening gate

Generate the final untracked batch under:

`artifacts/original-hybrid-subset-mix/`

The listening order is:

1. full twelve-layer delayed reference;
2. the four same-condition three-layer combinations; and
3. the three role-swapped four-layer combinations that passed.

The README states the selected layers, launch offsets, shared group gain,
active RMS, peak, hard-ceiling proportion, and any retained source limitation
for each file. It warns that digital level is not acoustic SPL and recommends
starting playback at low hardware volume.

Human listening answers only:

1. Which smaller combination comes closest to the fused, harmonized identity
   of the twelve-layer reference?
2. Does its moderate crest limiting add useful edge without turning into
   obvious distortion or noise?
3. Is any three- or four-layer result worth another iteration?

No candidate is selected for production, preserved in Git, mapped to controls,
or integrated into `Engine` without a later explicit user decision. The
rejected compact listening batch is documented as rejected and its ignored
generated files are deleted. Ordinary source, tests, and durable negative
research conclusions remain tracked.

## Verification and handoff

Before presenting the disposable batch, run:

- `cargo fmt --check`;
- `cargo test --all-targets --all-features`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo build --release`;
- `cargo audit`;
- `cargo deny check`;
- `cargo check --target aarch64-unknown-linux-gnu`;
- two fresh deterministic release generations and a comparison excluding only
  declared volatile workstation timing; and
- `git diff --check`.

Report exact pass/fail evidence and retain the known boundary that AArch64
compile evidence is not native Raspberry Pi performance, latency, polyphony,
or sound-quality evidence.
