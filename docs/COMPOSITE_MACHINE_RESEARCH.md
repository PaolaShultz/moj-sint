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

## 2026-07-23 hot audition, octave, bass, and punch correction

### Cause and consequence of the old level mismatch

The previous listening set was not level controlled. Every source layer was
prepared from its own whole-file RMS, candidate mix gains were selected for
headroom, and the final report again measured whole files. Delayed starts,
inactive score intervals, and tails therefore lowered some rows independently.
The synchronized reference also accumulated twelve schedules coherently at
sample zero, which was not how the separately launched player accident began.

The synchronized raw reference measured 0.070910 RMS, 7.088 dB above the
0.031355 delayed-launch estimate. Its advantage over reduced, role-separated,
harmonic-lattice, follower, and risky candidates was respectively 7.900,
8.347, 8.795, 9.204, and 12.052 dB. Relative to the more relevant delayed
estimate, those candidates were only 0.812, 1.259, 1.707, 2.116, and 4.964 dB
lower. The old order therefore strongly biased first impressions toward the
synchronized counterfactual.

### Declared output contract and prepared gains

The hot audition path measures the same 4.25-8.00 second musically active
window for every full-score render. Its target is 0.12 stereo RMS, with
0.10-0.14 accepted when one shared octave gain must expose register
differences. The hard sample-peak ceiling is 0.75. Gain is one explicit
constant multiplication after raw synthesis. There is no limiter, compressor,
clipper, per-file normalizer, peak maximizer, or emergency make-up stage.

| Topology | Full score | Held D1/D2/D3 | D1 pedal | Punch |
| --- | ---: | ---: | ---: | ---: |
| synchronized reference | 1.731418 (+4.768 dB) | n/a | n/a | n/a |
| delayed-launch estimate | 2.881948 (+9.194 dB) | n/a | n/a | n/a |
| reduced stack | 3.979082 (+11.996 dB) | 3.520000 (+10.931 dB) | 3.498000 (+10.876 dB) | 4.114000 (+12.285 dB) |
| role-separated | 4.638279 (+13.327 dB) | 4.110000 (+12.277 dB) | 3.678000 (+11.312 dB) | 4.493000 (+13.051 dB) |
| harmonic lattice | 4.906858 (+13.816 dB) | 3.640000 (+11.222 dB) | 3.760000 (+11.504 dB) | 4.609000 (+13.272 dB) |
| cross-topology follower | 5.123818 (+14.192 dB) | 3.760000 (+11.504 dB) | 3.974000 (+11.985 dB) | 5.030000 (+14.031 dB) |
| risky braid | 6.633916 (+16.435 dB) | 10.100000 (+20.086 dB) | 4.729000 (+13.495 dB) | 6.665000 (+16.476 dB) |

Full-score hot active RMS is 0.120000 within report precision and peaks are
0.431441-0.522820. D1 pedal active RMS is 0.122867-0.134671 with peaks
0.475787-0.531482. Punch active RMS is 0.114396-0.125566 with peaks
0.505186-0.581495. Raw renders and hashes remain separately available; the
synchronized and delayed raw hashes are sample-identical to the preceding
batch.

Digital level is not acoustic SPL and is not evidence of safe playback volume.
The hot batch must still be auditioned from a comfortable low monitor or
headphone setting.

### Same-topology octave and bass evidence

The five candidates were rendered as held MIDI 26 D1 (36.708099 Hz), MIDI 38
D2 (73.416199 Hz), and MIDI 50 D3 (146.832382 Hz). One fixed gain per topology
is reused across all three registers:

| Topology | D1 active RMS | D2 active RMS | D3 active RMS |
| --- | ---: | ---: | ---: |
| reduced stack | 0.125621 | 0.111266 | 0.100298 |
| role-separated | 0.119816 | 0.100051 | 0.117123 |
| harmonic lattice | 0.139794 | 0.130912 | 0.126510 |
| follower | 0.123321 | 0.100187 | 0.117170 |
| risky braid | 0.131109 | 0.100067 | 0.119978 |

No octave is independently normalized. Peaks are 0.353137-0.542087.
Correlation is 0.576155-0.966702, side/mid is 0.026448-0.290644,
low-band side/mid is 0.021902-0.450904, and mono fold loss is only
0.113-1.108 dB. The held renderer explicitly retains the dynamic follower and
pairwise nonlinear/DC-control mechanisms; an intermediate implementation that
omitted them made role-separated and follower rows more than 0.999 correlated
and was rejected. Final held pair similarities remain below 0.974.

The octave results expose rather than conceal weak pitch behavior. In
particular, risky-braid D1 measures its nominal 36.7 Hz projection at
-60.520 dB while its octave and third-harmonic projections are -23.399 and
-23.209 dB. That is severe weak-fundamental bass behavior, not a successful D1
claim. Across octave rows, the conservative worst-constituent 8x residual is
-1.862 to -0.341 dB. These very high residuals, the earlier dual-body
MIDI-36 result, and the full-score residuals remain known alias/sample-rate
limitations. They are not evidence of inaudibility.

Every full musical candidate also has one explicit D1 pedal source. The punch
version envelopes that source before its stereo matrix and composite sum; it
does not envelope or normalize the master mix. Matching D1 held, pedal, and
punch mono diagnostics are placed after all stereo listening files.

### Research ADSR sweep

The sample-accurate research ADSR owns prepared integer stage lengths, remains
finite and bounded in [0, 1], retriggers deterministically from zero, and
releases continuously from its current level. Allocation tests cover its
sample path. It does not alter either reconstruction reference.

The formal sweep tested:

- 1/70/0.55/100 ms, rejected because onset-to-sustain ratio was
  0.950-0.954, below 1.05;
- 3/110/0.55/180 ms, retained as the bounded engineering candidate; and
- 5/160/0.75/250 ms, rejected because the ratio was 0.939-0.942.

An earlier 3/110/0.65/180 revision was also rejected during report review:
its 48 kHz ratio was only about 0.98, so it mostly lowered the source rather
than establishing the required onset/body contrast.

For the retained setting, onset-to-sustain ratio is 1.094-1.098, maximum
sample jump is 0.0225-0.0339 in the isolated hot probe, release-event jump is
0.000250-0.011936, and measured attack/decay settling are 3/113 ms. The report
also retains 10/50/100/250 ms RMS and peak, low-band transient RMS,
fundamental-through-decay change, 180 ms tail, peak, and crest values.
Measurements may establish bounded contrast; they cannot declare the result
musically punchy.

### Final disposable batch and open decision

The ignored batch now contains 54 WAVs in eight README sections: hot delayed
reference, hot synchronized counterfactual, five hot full-score candidates,
fifteen D1/D2/D3 held files, five D1 pedal versions, five punch versions,
fifteen mono diagnostics, and seven raw engineering renders. Reports cover
gain policy, octave behavior, ADSR sweep, allocation, residuals, similarity,
hashes, stereo/mono, bands, projections, reconstruction regression, forbidden
processors, and workstation cost.

The exact next human decision is whether any topology retains the fused
accidental identity at controlled level; whether its D1 behavior is musically
usable on headphones, speakers, and mono; and whether the single retained
envelope produces desirable punch. No topology, octave behavior, or ADSR has
been musically accepted.
