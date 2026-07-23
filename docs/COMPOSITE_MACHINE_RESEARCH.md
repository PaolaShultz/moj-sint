# Composite Machine Research Checkpoint

Date: 2026-07-23

## Status

The accidental simultaneous-player listening result is the first strongly
positive direction in Moj Sint: the combined sound was described as huge,
fused, harmonized, and interesting enough to inspire wanting the machine.

This is evidence for heterogeneous mechanisms acting together. It is not an
accepted production sound, preset, macro route, graph, or causal explanation.
The final batch remains an open human listening gate.

## What the accident contained

The twelve files crossed three complete hybrid families with four conditions:

- cross-coupled machine, spectral shadow, and dual resonant body;
- one 10-second MIDI-38 voice;
- one 12-second MIDI 50/53/57 held chord;
- one 16-second stereo four-chord progression; and
- the same deterministic 16-second progression folded to mono.

The equal-tempered frequencies used by the scheduler are:

| MIDI | Note | Frequency (Hz) |
| ---: | --- | ---: |
| 38 | D2 | 73.416192 |
| 45 | A2 | 110.000000 |
| 46 | B-flat2 | 116.540947 |
| 48 | C3 | 130.812783 |
| 50 | D3 | 146.832384 |
| 52 | E3 | 164.813778 |
| 53 | F3 | 174.614116 |
| 55 | G3 | 195.997718 |
| 57 | A3 | 220.000000 |

The progression is D minor 50/53/57, B-flat major 46/50/53, C major
48/52/55, and A minor 45/48/52.

### Synchronized counterfactual

If every file starts at sample zero, the exact synthesis-voice inventory is:

| Time | Total voices | MIDI counts |
| --- | ---: | --- |
| 0-4 s | 30 | 38x3, 50x9, 53x9, 57x9 |
| 4-8 s | 30 | 38x3, 46x6, 50x9, 53x9, 57x3 |
| 8-10 s | 30 | 38x3, 48x6, 50x3, 52x6, 53x3, 55x6, 57x3 |
| 10-12 s | 27 | 48x6, 50x3, 52x6, 53x3, 55x6, 57x3 |
| 12-16 s | 18 | 45x6, 48x6, 52x6 |

Thus the first four seconds contain three D2 sources plus nine separate
D-minor ensembles: 30 synthesis voices before internal partials,
subharmonics, resonators, register events, or stereo states.

### Corrected delayed-launch interpretation

The real files were not synchronized. Each was spawned in a separate player
process with a noticeable delay. The WAVs have no launch timestamps, and no
player/process log is available, so exact historical offsets and order cannot
be recovered.

The reproducible estimate uses filename order and start offsets:

```text
0, 180, 420, 710,
1030, 1380, 1760, 2170,
2610, 3080, 3580, 4110 ms
```

It ramps from 1 voice at 0 ms to 4 at 180 ms, 7 at 420 ms, 10 at 710 ms,
14 at 1380 ms, 20 at 2170 ms, 24 at 3080 ms, 27 at 3580 ms, and 30 only at
4110 ms. From then onward, independently shifted progression changes and file
ends continually alter the inventory until the final three-voice layer ends at
20.110 seconds. The generated `frequency-timeline.tsv` records every interval.

This estimate is intentionally not called exact. Its purpose is to isolate
plausible density ramp, phase dispersion, beating, and asynchronous ending
effects.

## Why it may perceptually fuse

Plausible contributors are:

- D2/D3 octave reinforcement and a D-minor-centered fixed chord;
- later progressions sharing tones with that fixed material;
- heterogeneous spectra filling different gaps;
- independent resonant, nonlinear, register, and modulation state;
- accumulated harmonic and subharmonic energy;
- simultaneous stereo and mono-fold copies increasing mid density while
  retaining side structure;
- launch skew dispersing phases and creating slow beating;
- density changes when 10- and 12-second sources end; and
- repeated chord tones becoming partial groups in one compound object rather
  than separate keyboard gestures.

No factor is declared causal. The two references, reduced graph, role
separation, harmonic lattice, typed follower, and nonlinear braid are the
ablation hypotheses.

## Implementation boundary

`src/composite_machine.rs` constructs every layer by internally rendering Moj
Sint hybrid DSP. It never launches processes or reads WAVs during rendering.
Construction prepares fixed sample storage, start/stop samples, source gains,
fades, matrices, seeds, and score state. `CompositeMachine::sample` only reads
that fixed storage and mixes a bounded layer set; allocation tests cover the
sample path.

The synchronized reference uses the exact old conditions, seeds, fades, and
per-file RMS/peak preparation, followed by a fixed 0.16 composite headroom
gain. A one-off external WAV-sum subtraction verified the internal reference:
the difference is -90.31 dB peak and -130.35 dB RMS over 768,000 frames. This
residue is consistent with floating addition order. WAV rereading is not part
of the renderer.

No final limiter, emergency normalizer, or peak maximizer is used.

## Implemented final topologies

1. **Synchronized reference:** all twelve original schedules at sample zero;
   controlled counterfactual.
2. **Delayed-launch estimate:** the same twelve layers under the fixed
   0-4110 ms stagger; closest reproducible model, not historical fact.
3. **Reduced heterogeneous stack:** removes all redundant progression-mono
   copies and retains cross single, spectral held chord, cross progression,
   and dual progression.
4. **Role-separated machine:** cross owns a 16-second centered D2 body,
   spectral owns the held harmonic object, and dual body owns changing
   transition/resonant energy.
5. **Harmonic lattice:** centered D2 plus individual lower, middle, and upper
   progression relationships assigned across all three mechanisms.
6. **Cross-topology follower:** the cross-family mid envelope drives bounded
   spectral gain and the dual family's side placement. The measured control
   range is 0.75-1.25.
7. **Risky nonlinear braid:** three full heterogeneous progression bodies feed
   a low-level pairwise multiplicative exchange. A prepared 8 Hz internal DC
   control removes junction bias. A dry linear path retains chord readability.

## Iterations, rejections, and redesigns

### Iteration A

The first risky candidate reused the role-separated dry roles and added one
cross/dual multiplicative junction. It was rejected:

- correlation with role-separated was 0.984317, only 0.000683 inside the
  preregistered 0.985 near-duplicate bound;
- composite residual was -4.231 dB, the worst final-candidate value;
- DC was 0.000081873, much higher than the other candidate outputs; and
- the dry topology was not structurally original enough.

The temporary batch was deleted. The risky graph was replaced by three full
progression bodies with pairwise `a*b`, `b*c`, and `c*a` exchange. The follower
was also completed so the measurement controls both a second-family gain and a
third-family stereo field.

### Iteration B

The new risky topology was distinct: correlation with role-separated fell to
0.313040. However, its total difference products were
-48.280/-49.432/-48.670 dB while the weakest first-segment target was
-38.792 dB. The 9.5 dB separation failed the predeclared 30 dB junction rule.

The temporary batch was deleted. A linear-control subtraction was added so
reports isolate products caused by the junction, and exchange depth was reduced
from 0.75/0.55 center/side coefficients to 0.06/0.045.

### Iteration C

The isolated junction products are now -70.224, -71.373, and -70.642 dB.
The weakest first-segment target is -38.943 dB, leaving 31.281 dB margin.
Junction peak remains measurable at 0.005673. Correlation with role-separated
is 0.359106, and the highest correlation between any two final candidates is
0.765512.

No final candidate has an inert mute ablation. Final layer mute differences
range from -2.485 to -9.334 dB relative to the complete candidate. In the
references, progression and progression-mono copies each remain active, but
they are exact deterministic mid-density redundancy; the reduced graph removes
all three mono copies and six other layers without claiming that metrics alone
prove the accident's musical identity survived.

## Final engineering evidence

Across the two references and five final candidates:

- sample peak: 0.066021-0.289289;
- RMS: 0.017705-0.070910;
- maximum absolute DC: 0.000006314;
- crest factor: 3.729-4.775, except no quality threshold is inferred;
- correlation: 0.753955-0.966233;
- side/mid energy: 0.029964-0.147105;
- low-band side/mid energy: 0.026599-0.103616;
- mono/stereo RMS ratio: 0.933681-0.985347;
- maximum final-candidate scheduled-target spread: 12.694 dB;
- delayed-reference worst target spread: 33.835 dB, still within the
  preregistered 36 dB bound; and
- follower range: 0.75-1.25.

Every output is finite, below the 0.8 headroom ceiling, genuinely stereo when
claimed, non-silent in mono, deterministic, and free of sample-path allocation.

The conservative 8x residuals are severe and descriptive:

- synchronized reference -6.589 dB;
- delayed estimate -13.296 dB;
- reduced stack -6.359 dB;
- role-separated -5.012 dB;
- harmonic lattice -10.010 dB;
- follower -8.209 dB; and
- risky braid -6.194 dB, with isolated junction residual -3.670 dB.

None is called alias-clean or sample-rate-invariant. The earlier dual-resonant
MIDI-36 -2.889 dB result also remains a known source limitation.

Prepared sample-loop workstation cost is 7.5-21.9 ns/frame in this run. It
excludes internal offline DSP preparation and is not Raspberry Pi, callback,
latency, polyphony, or sound-quality evidence.

## Typed routing implications

The active follower suggests:

```text
AudioStereo -> Mid -> EnvelopeMeasurement
EnvelopeMeasurement -> bounded ControlGain
EnvelopeMeasurement -> bounded StereoSideScale
```

The risky candidate suggests an explicitly marked nonlinear junction:

```text
AudioMono x AudioMono -> ProductAudio
ProductAudio -> DCControl -> bounded StereoMatrix
```

The layer scheduler also exercises note/chord score, deterministic seed/phase,
sample-offset, gain, stereo matrix, and named probe concepts. These are
candidate typed nodes/ports, not approval to extract the generic compiler.

## Human listening gate

Listen in this order:

1. synchronized counterfactual;
2. delayed-launch estimate;
3. reduced heterogeneous stack;
4. role-separated machine;
5. harmonic lattice;
6. cross-topology follower;
7. risky nonlinear braid; and
8. matching mono diagnostics only after the stereo identities are understood.

The precise next action is to record which complete candidates retain the
accident's fused identity and which individual connections are audibly useful.
Only then decide whether to preserve a specific result, extract typed nodes, or
route any mechanism toward production.
