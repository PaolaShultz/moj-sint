# Composite Hot Audition, Octave, and Punch Design

Date: 2026-07-23

## Goal and scope

Correct the existing composite-machine listening presentation without changing
its raw engineering references. Add controlled D1/D2/D3 same-topology
comparisons, D1 bass/pedal versions of the existing musical behavior, and one
bounded punch-envelope hypothesis per surviving composite topology.

This remains isolated offline research. Production `Engine`, presets, stable
macros, JACK, ALSA, SHR-DAW, and the hybrid source mechanisms are unchanged.
Automated evidence may reject unsafe or ineffective output preparation, but
only human listening can decide whether a topology or envelope is musical.

## Diagnosis and alternatives

The old lab prepared each source file to its own whole-render RMS and then used
fixed composite mix gains. Final RMS was also measured over whole outputs,
including delayed-launch silence and tails. The synchronized counterfactual
placed all twelve schedules at sample zero, adding coherent density that the
separately launched accident did not have. It measured 0.070910 RMS while the
delayed estimate measured 0.031355 and the five candidates measured
0.017705-0.028555, a 7.09 dB delayed-reference deficit and 7.91-12.06 dB
candidate deficit relative to the synchronized counterfactual.

Three approaches were considered:

1. Change the existing layer gains. This obscures the reconstruction and risks
   invalidating the raw reference hashes.
2. Normalize or limit each generated file. This violates the explicit-output
   requirement, hides octave weakness, and changes dynamics.
3. Preserve raw synthesis and add a declared fixed output-gain stage for
   audition renders. This is selected because it keeps the engineering
   references sample-identical and makes every audible gain reviewable.

## Output contract

All hot files use one explicit prepared linear gain per topology. Gains are
derived once from the stereo RMS of the common 4.25-8.00 second active-body
window, targeting 0.12. The gain is reduced only when required to keep every
sample at or below 0.75. The committed policy is a table of constants, not a
runtime per-file maximizer. A candidate that still cannot reach 0.10 without
crossing 0.75 must be redesigned or documented as peak-constrained.

The acceptable active-body RMS range is 0.10-0.14. There is no limiter,
compressor, clipper, emergency normalization, or hidden make-up gain.
Full-render RMS remains reported as secondary context. Raw and hot hashes,
linear/dB gain, peak, RMS, DC, crest, jump, bands, projections, onset/body,
stereo, low-frequency side/mid, and mono evidence are retained.

The delayed-launch reconstruction is listening reference 1. The synchronized
reconstruction is a counterfactual in position 2. Digital sample level is not
acoustic SPL and is not evidence of safe playback volume.

## Octave and bass boundary

The five surviving candidate topologies are rendered as four-second held notes
at MIDI 26 (D1, 36.708096 Hz), MIDI 38 (D2, 73.416192 Hz), and MIDI 50 (D3,
146.832384 Hz). Each topology retains its family inventory, matrices, and mix
relationships. The same committed candidate output gain is used at all three
pitches; no octave receives independent normalization. The held-note active
window is 0.25-3.75 seconds.

Each topology also receives one sixteen-second musical version whose existing
score is preserved while its low anchor is a controlled D1 pedal. Topologies
without a prior dedicated anchor receive one explicit D1 source rather than a
post-mix bass boost. Stereo and mono versions measure low-band mid/side energy
and cancellation.

## Research ADSR

`ResearchAdsr` is a sample-accurate fixed-state envelope owned only by the
composite lab. Construction validates and prepares integer stage lengths and
per-stage slopes. Sampling performs bounded arithmetic only and allocates
nothing. Note-on retriggers deterministically from zero; note-off begins a
continuous release from the current level. Attack, decay, sustain, and release
boundaries clamp exactly, remain finite, and never jump outside [0, 1].

The envelope is applied to each scheduled D1 pedal source before stereo
matrixing and composite summing. References are never enveloped. The
engineering sweep covers distinct bounded settings across attack 1-5 ms,
decay 70-160 ms, sustain 0.55-0.75, and release 100-250 ms. It rejects settings
that breach the hot peak contract, fail continuity, or fail to create measured
onset-to-sustain contrast. Exactly one retained setting is rendered per
topology; metrics do not call it punchy.

Reports include 10/50/100/250 ms RMS and peak, onset-to-sustain ratio, measured
attack and decay settling, maximum jump, low-band transient energy,
fundamental stability during decay, release continuity/tail duration, and hot
peak/crest.

## Output and testing

Generation first targets fresh temporary directories. Deterministic reports and
WAVs must compare byte-for-byte except workstation cost. Only after all
rejection rows pass is `artifacts/composite-machine-lab/` replaced.

The listening README order is:

1. hot delayed-launch reference;
2. hot synchronized counterfactual;
3. five hot full-score candidates;
4. D1/D2/D3 held-note topology comparisons;
5. five D1 bass/pedal musical versions;
6. five punch-envelope versions;
7. mono diagnostics;
8. raw engineering renders.

Tests lock the raw reconstruction hashes, hot RMS range/peak ceiling, common
gain across octaves, finite/bounded/allocation-free deterministic ADSR,
pre-sum envelope placement, absence of any limiter/normalizer path, complete
reports, and deterministic release generation. Conservative eight-times
residual and scalar x86_64 workstation cost remain descriptive, not Pi or
callback evidence.
