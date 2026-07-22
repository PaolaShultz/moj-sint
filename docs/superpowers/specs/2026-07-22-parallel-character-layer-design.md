# Parallel Character Layer Design

## Scope and decision

This milestone implements one monophonic output-layer experiment: an
untouched per-voice dry anchor plus one bounded nonlinear character return. It
does not add a subharmonic, feedback, diffusion, post-mix saturation, a live
host, SIMD, or an accepted factory macro mapping.

Three exact small graphs were compared before implementation:

1. **Full-band symmetric parallel saturation:**
   `x -> drive -> odd soft clip -> remove linear component -> return gain`, in
   parallel with dry `x`. This is the smallest graph and is intrinsically
   zero-centered, but it presents every high partial in the saw/square base to
   the nonlinearity. It therefore has the greatest aliasing and high-note
   hardening risk and is most likely to sound like generic full-band distortion.
2. **Band-limited symmetric generated-residual layer:**
   `x -> low-pass -> drive -> odd soft clip -> remove linear component -> DC
   blocker -> bounded return`, in parallel with dry `x`. This is selected. It
   preserves the base waveform exactly at zero return, confines the nonlinear
   input bandwidth, adds dense odd-order products rather than sparse selected
   sines, and avoids brightening the complete signal merely to expose a
   difference.
3. **Band-pass asymmetric exciter:**
   `x -> high-pass -> low-pass -> biased soft clip -> remove linear component
   -> DC blocker -> low-pass -> bounded return`, in parallel with dry `x`. The
   bias supplies even and odd products, but adds a second cutoff, bias policy,
   stronger DC dependence, and a greater risk of a thin or harsh result. It is
   deferred until the symmetric branch establishes whether parallel generated
   harmonics have musical value at all.

A per-voice pitch-locked half-frequency oscillator would give the clearest
subharmonic correctness and future chord behavior. Period division or post-mix
tracking would add onset, release, and chord ambiguity. All three subharmonic
forms are deferred because they answer a different low-register question and
must not be combined with the first nonlinear listening gate.

## DSP topology and placement

Let `x` be the existing per-voice `SHAPE`/`COLOR` output with the rejected
harmonic selector bypassed. One four-pole cascade of scalar one-pole sections
limits the branch presented to the nonlinearity. Coefficients are prepared from
sample rate outside the sample loop. The static odd law is an independently
implemented cubic soft clip with a constant continuation:

```text
S(u) = u - u^3/3,  |u| <= 1
S(u) = sign(u) 2/3, otherwise
```

The generated component is `S(drive * b) / drive - b`, where `b` is the
band-limited branch. Subtracting the linear component makes the return vanish
as drive tends to zero and prevents the parallel path from becoming an EQ-like
copy of the dry signal. A one-pole DC blocker follows the nonlinear residual;
theoretical odd symmetry does not replace a measured DC guard for asymmetric
source waveforms. The final voice sample is:

```text
y = x + return_gain * dc_block(generated_component)
```

`return_gain` is explicitly bounded. `EDGE` is the candidate drive/intensity
control and `COUPLE` is the candidate dry-to-character interaction/return
control, both through the existing 10 ms smoothers. `COUPLE=0` is bit-exactly
independent of `EDGE` and is the research baseline. These mappings remain
experimental even if automated checks pass; only user listening can accept
them as useful stable-role implementations.

The branch is per voice before voice mixing. That retains per-note state,
avoids inter-voice saturation products, and does not lock later polyphony into
post-mix pitch ambiguity. Post-mix saturation will be reconsidered only in a
separate intermodulation experiment. A hybrid with per-voice layers and a
post-mix character bus remains possible because the engine mix boundary is
unchanged.

## Antialiasing selection

BLAMP is not the first comparison for this smooth-slope memoryless law; it is
better matched to correcting known discontinuities such as fold or reset
events. The implementation will compare direct evaluation, first-order ADAA,
and two-times oversampling against a high-rate offline reference on notes
36/60/84/96 at moderate and strong drive. The production path will use the
least costly method within 3 dB of the best worst-case candidate, provided its
alias/error is at most -60 dB at moderate drive and -50 dB at strong drive for
every measured note. If no candidate meets those limits, the branch bandwidth
or drive range must be reduced before listening files are produced. Ties prefer
direct evaluation, then ADAA, then two-times oversampling. The measurements
will be recorded rather than assuming that filtering alone is sufficient.

First-order ADAA follows the independently implemented antiderivative method
described by Stefan Bilbao, Fabián Esqueda, Julian D. Parker, and Vesa
Välimäki, “Antiderivative Antialiasing for Memoryless Nonlinearities,” *IEEE
Signal Processing Letters* 24(7), 2017, DOI 10.1109/LSP.2017.2675541. The
project uses the published mathematics and citation, not third-party source,
prose, figures, presets, or samples.

## Evidence and listening gate

Test-first DSP checks cover invalid construction, deterministic reset, finite
and bounded output, DC removal, direct-bypass identity, rapid drive/return
movement, smoothing, and allocation-free primitive and engine render paths.
Offline measurements record peak, RMS, DC, harmonic distribution, fundamental
retention, alias/error against a high-rate reference, level growth, and scalar
workstation cost. A two-tone post-mix probe records what intermodulation would
be introduced by the deferred placement; it does not place the production
nonlinearity after the mixer.

The first listening gate contains exactly nine loudness-matched stereo WAVs:
dry, moderate, and intentionally strong settings at MIDI notes 36, 60, and 84.
All use `SHAPE=0.5`, `COLOR=0.5`, one voice, and `COUPLE=0` for dry. Filenames
state note and condition. Automated evidence can reject instability, aliasing,
pitch loss, ineffective travel, or unsafe growth, but cannot establish musical
usefulness. If listening suggests minor EQ, ordinary distortion, or no useful
identity, the branch is rejected rather than expanded into a larger matrix.

All timing is scalar x86_64 workstation development evidence. It is not native
Raspberry Pi callback, latency, polyphony, or sound-quality evidence.
