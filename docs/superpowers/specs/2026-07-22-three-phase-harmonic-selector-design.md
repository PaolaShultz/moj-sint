# Three-Phase Harmonic Selector Design

## Scope and selection

This milestone implements one monophonic oscillator-system experiment: a
shared-phase 0/120/240-degree harmonic selector. It does not implement the live
host, polyphony expansion, SIMD, a vintage-instrument clone, or a factory
preset.

Three candidates were considered:

1. Nonlinear modulator preprocessing can expose non-commuting phase/fold order,
   but its first useful experiment also needs a modulation-phase API, DC
   blocking, and nonlinear antialiasing policy.
2. A shared three-phase bank gives a pitch-locked topology, an exact
   roots-of-unity cancellation invariant, and bounded scalar work with one
   phase state. This is the selected smallest milestone.
3. A two- or three-node PM/AM/feedback graph offers a larger identity space,
   but combines operator ratios, edge ordering, feedback stability, and
   sideband aliasing before a simpler connection-order primitive is proven.

## DSP graph and macro intent

Each voice owns one recursive sine phase source. From its sine/cosine state it
derives three simultaneous taps at 0, 120, and 240 degrees using fixed algebraic
coefficients. No per-sample trigonometric setup is added.

The equal sum of the three linear taps is approximately zero. Applying the same
cubic operation to each tap and summing isolates the third harmonic; the
implementation normalizes that sum to a bounded unit-amplitude target. `EDGE`
crossfades from the zero-degree fundamental tap to this equal nonlinear
three-tap result. `COUPLE` crossfades the existing `SHAPE`/`COLOR` oscillator
path toward the selector. Both controls use the existing 10 ms smoother and a
bounded algebraic gain compensation. At the reference preset's middle
`COUPLE`, `EDGE` must affect most of its travel; at zero `COUPLE`, the existing
oscillator remains an intentional unchanged baseline.

The prepared third-harmonic route remains full while its target is at or below
40% of sample rate, tapers to zero before 48% of sample rate, and then falls
back to the fundamental tap. This keeps the cubic third from folding above
Nyquist at high MIDI notes without adding sample-rate-dependent branches or
transcendental work to the sample loop.

The bank is per voice even though this phase renders one voice for evidence.
Frequency preparation and reset happen on note start. The sample path remains
finite, deterministic, allocation-free, scalar, and independent of JACK,
ALSA, files, processes, and clocks.

## Evidence and acceptance

Unit tests first establish exact deterministic reset, finite/bounded samples,
the linear cancellation invariant, third-harmonic selection, and no allocation.
Engine tests then establish `EDGE`/`COUPLE` smoothing, meaningful adjacent
travel differences at the reference operating point, finite bounded rapid
movement, and no render allocation.

The offline lab renders loudness-matched baseline/selector A/B files at MIDI
notes 36, 60, and 84 with low/mid/high `EDGE` and `COUPLE` positions. Its
manifest records peak, RMS, DC, sample hash, fundamental and third-harmonic
levels, non-harmonic alias/error energy, and pitch-retention evidence. A release
workstation loop reports relative scalar cost as development evidence only.

Automated checks may establish that the topology is controllably different and
reject numerical defects. They do not establish musical usefulness. Human
listening acceptance remains with the user, and no Raspberry Pi performance,
latency, sound-quality, or safe-polyphony claim is made.

## Listening outcome

The user listened to all generated conditions and rejected the topology as too
close to pure sine material, with no useful distinctive identity. The generated
artifact corpus was removed. The implementation remains only as a bounded
negative-result baseline and testable harmonic-analysis path; its `EDGE` and
`COUPLE` mappings are not accepted for a factory preset.

## Documentation and licensing

The implementation is independently derived from the phase identities and the
project's existing research register. Existing authors, publications, URLs,
and licensing notes remain intact. No third-party DSP source, presets, samples,
prose, or figures are imported.
