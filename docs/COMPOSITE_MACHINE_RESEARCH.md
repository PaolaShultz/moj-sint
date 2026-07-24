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

## 2026-07-23 compact composite power experiment

### Correction and scope

The previous 54-WAV hot batch remained valuable engineering evidence but was
not the requested reduction: its successor candidates still contained three
or four complete hybrid layers and the references contained twelve. It also
forbade clipping and retained a 0.75 ceiling. The new isolated
`compact_composite` boundary preserves both reconstruction hashes and replaces
only the disposable listening presentation.

Ten primary experiments now use at most three simultaneous mechanisms:

| File / experiment | Role | Mechanisms | Fixed source gains | Drive and output |
| --- | --- | --- | --- | --- |
| `02` Deep Rotor | sustained D1 | table sub + register rotor | 0.94, 0.44 | rational 3.4, output 0.92 |
| `03` Phase Forge | playable D3 | nonlinear phase interaction | 1.00 | cubic 2.9, output 0.96 |
| `04` Evolving Lattice | evolving D2 | spectral bank + register rotor | 0.82, 0.42 | rational 3.7, output 0.90 |
| `05` Pedal Monolith | D1 pedal | table sub + resonant body + register rotor | 0.82, 0.72, 0.46 | rational 3.6, output 0.92 |
| `06` D2 Braid | D2 bass | nonlinear phase interaction + table sub | 0.72, 0.62 | cubic 3.2, output 0.96 |
| `07` Clipped Thump | D1 thump | pitch-drop table oscillator | 1.00 | hard clip 8.0, output 1.00 |
| `08` Resonant Thump | D1 thump | resonant body + table sub | 0.74, 0.72 | rational 6.0, output 0.94 |
| `09` Synthetic Kick | D1 kick | pitch-drop table oscillator + noise + resonant body | 1.00, 1.35, 0.46 | hard clip 9.5, output 1.00 |
| `10` Struck Comb | struck D3 | noise transient + excited comb | 0.42, 0.92 | cubic 4.2, output 0.94 |
| `11` Context Relay | eight-event context | table sub + spectral bank + register rotor | 0.67, 0.62, 0.62 | rational 3.9, output 0.92 |

Every row uses 12 Hz pre/post-drive DC control and a declared 0.999 written
ceiling. Stereo widths and exact envelope/tone settings are in `settings.tsv`.
No gain changes across pitch or tone, and no per-file normalization occurs.

### Envelopes and event evidence

Sustained machines use 12/180/0.86/220 ms ADSR with a 3700 ms gate. Evolving
Lattice uses 18/240/0.88/300 ms and a 5650 ms gate. Context Relay uses
4/90/0.72/180 ms per 760 ms event.

The one-shot settings are:

- Clipped Thump: 0.5/95/0.32/260 ms, 210 ms gate, 19-semitone drop over 95 ms;
- Resonant Thump: 1.2/140/0.42/340 ms, 270 ms gate, 7-semitone drop over 130 ms;
- Synthetic Kick: 0.25/72/0.18/290 ms, 160 ms gate, 28-semitone drop over 78 ms; and
- Struck Comb: 0.4/120/0.28/1150 ms, 190 ms gate, no pitch drop.

Their first 10/25/50/100/250 ms RMS values are respectively:

- Clipped Thump: 0.900/0.907/0.907/0.885/0.839;
- Resonant Thump: 0.892/0.904/0.874/0.866/0.825;
- Synthetic Kick: 0.948/0.912/0.919/0.861/0.798; and
- Struck Comb: 0.619/0.609/0.466/0.337/0.214.

Declared pitch trajectories are 110.000 to 36.708 Hz, 55.000 to 36.708 Hz,
184.997 to 36.708 Hz, and a stable 146.832 Hz. Measured -60 dB
decay/tail times are 485/551, 631/689, 457/517, and 581/821 ms. Full-file
clipped proportions are 8.015%, 6.231%, 4.854%, and 0%. Maximum sample jumps
are 0.132, 1.444, 1.996, and 1.881. All return-to-zero values are zero at the
reported precision; absolute DC is below 2.6e-8 for these files.

### Loudness floor, control retention, mono, and ablation

The hot delayed reference remains first and measures active RMS 0.120000 and
perceptual proxy -19.535 dB. Primary active RMS spans 0.169-0.887; the
struck event is +2.987 dB RMS/+6.919 dB proxy above the reference, while the
other events and all sustained/musical rows are much hotter. Primary full-file
clipping is deliberate: Deep Rotor 28.070%, Phase Forge 21.470%, Evolving
Lattice 9.647%, Pedal Monolith 28.478%, D2 Braid 29.208%, Clipped Thump
8.015%, Resonant Thump 6.231%, Synthetic Kick 4.854%, Struck Comb 0%, and
Context Relay 27.806%.

Minimum/median/maximum tone sweeps retain RMS within 0.002522-0.502115 dB and
the perceptual proxy within 0.034589-0.461135 dB. D1/D2/D3 use one fixed gain
policy.
All bass-bearing primary mono folds lose at most 0.001 dB; the widest file,
Struck Comb, loses 0.048 dB. Low side/mid is at most 0.0181 and no projected
fundamental loses level after fold.

Every final mechanism has a mute hash and active difference. Full differences
span -22.588 to +0.395 dB. Pedal Monolith's resonant body is the quietest
whole-file ablation but changes the first 250 ms by -10.764 dB. The final
Clipped Thump has one mechanism because its prior noise transient was removed
at -28.9 dB whole/-27.4 dB onset difference.

### Rejections and limitations

Other rejected/revised points:

- weak register/resonator/noise gains were strengthened until mute evidence
  passed; exact iteration values are in `RESEARCH.md`;
- the first pitch-drop scheduler used a curved multiplier that did not land
  exactly on the declared base frequency at the final scheduled sample; it was
  rejected in report review and replaced by a prepared linear frequency
  decrement whose boundary sample is regression-tested;
- output without final DC control produced +0.083/+0.036 DC and was rejected;
- absolute-note Context Relay pitch rows were rejected and replaced by a
  score transposed around the requested D1/D2/D3 base;
- sweep grids and near variants remain reports, never listening files; and
- no limiter was added to make a weak design pass.

The 4x conservative residual spans -42.181 to -0.004 dB. Struck Comb is the
worst at -0.004 dB; Pedal Monolith (-3.292), Resonant Thump (-2.262), and
Synthetic Kick (-5.694) are also severe. Hard clipping is intentional but
does not make these paths alias-clean or sample-rate invariant. Fundamental
projections for percussive material are window-sensitive and are not pitch
perception measurements.

### Final listening order and open questions

Use `artifacts/composite-machine-lab/README.md`: hot delayed reference, files
`02` through `11`, then the six `12_mono_*` diagnostics. The batch is
intentionally hot; start with playback volume low. Digital level is not
acoustic SPL.

The subsequent human listening verdict rejected every compact candidate. None
worked musically, and the presentation's 4.854-29.208% full-file clipping was
an excessive interpretation of the request for a slightly hotter signal with
occasional hard-limited crests. The generated batch was deleted. The source,
tests, and evidence remain as a durable negative result.

The rejected listening questions were:

1. Which files are genuinely powerful rather than merely clipped or loud?
2. Which one-, two-, or three-mechanism identities are distinguishable and
   special?
3. Does Evolving Lattice change tone without perceptual power loss?
4. Which D1/D2 bodies retain pitch and weight on headphones, speakers, and
   mono?
5. Are either thump and the kick purpose-designed musical events, or only hot
   engineering transients?
6. Does Struck Comb's identity justify its severe sample-rate residual?
7. Does Context Relay demonstrate useful combination behavior?

No result is selected, preserved outside the disposable batch, routed into
production, or claimed musically successful.

## 2026-07-24 exact original-layer subset experiment

The corrected experiment does not invent smaller replacement machines. It
selects three or four exact layers from the original twelve-file pool and
retains their DSP, schedules, seeds, fades, channel construction, preparation,
and relative delayed-launch spacing. The candidates are:

1. all three singles;
2. all three held chords;
3. all three stereo progressions;
4. all three mono progressions;
5. Cross single + Spectral chord + Dual progression + Cross mono progression;
6. Spectral single + Dual chord + Cross progression + Spectral mono progression;
7. Dual single + Cross chord + Spectral progression + Dual mono progression.

There is no compressor, automatic limiter, soft saturator, maximizer, filter,
new envelope, pitch change, nonlinear cross-coupling, or per-file
normalization. The only presentation stage is shared fixed linear gain into a
static -0.3 dBFS hard ceiling. Three-layer candidates share gain 17.5;
four-layer candidates share gain 16.5; the twelve-layer delayed reference uses
gain 10.0.

All seven candidates pass the predeclared gate. Active RMS is
-10.813 to -10.090 dBFS and hard-ceiling contact is 0-0.292%. The reference
measures -10.122 dBFS with 0.480% ceiling contact. Tonal coverage is 100% for
every scheduled segment. No candidate is classified as noise-like:
magnitude-spectrum flatness is 0.230-0.468 and every tonal projection passes.
Mono loss is at most 0.509 dB, correlation is 0.804-1.000, absolute DC is
below 0.000126, and maximum adjacent-sample jump is below 0.165.

The eight-times post-sum hard-ceiling residual is -4.749 dB for singles,
-8.700 dB for chords, -3.706 dB for stereo progressions, -4.793 dB for mono
progressions, -6.154 dB for the Cross anchor, -8.820 dB for the Spectral
anchor, and -4.041 dB for the Dual anchor. These are retained engineering
limitations, not alias-clean or sample-rate-invariant claims.

Two fresh release batches are byte-identical except
`workstation-cost.txt`. Their deterministic aggregate SHA-256 is
`5001495920bb0fd23034ee9b91b13b6e87bb5470b326ed2c36982158ed716ddc`.
The raw reconstruction hashes remain `be3ea0fdd66b4472` synchronized and
`c919cb57920c520f` delayed.

The ignored disposable listening batch is
`artifacts/original-hybrid-subset-mix/`. Human listening must compare the
twelve-layer delayed reference with the seven smaller combinations in README
order. Automated evidence only confirms that obvious level, clipping, tonal,
noise-like, residual, DC, discontinuity, stereo/mono, finite, and tail
failures were not presented.

That batch was subsequently rejected as a timing interpretation. Rebased
second-scale offsets made layers enter as separate audible events rather than
varying the sum inside one voice. Its ignored artifact directory was removed.

## 2026-07-24 coherent fixed-microdelay composite experiment

The seven exact-source memberships remain unchanged. Only their combination
boundary changes:

- three-layer files use fixed `0/2/5 ms` offsets;
- four-layer files use fixed `0/4/9/15 ms` offsets;
- the twelve-layer orientation reference uses
  `0/1/3/4/5/7/8/9/11/12/14/15 ms`;
- every onset occurs within the common 25 ms attack;
- equal-power layer trim is applied before summation;
- one post-sum stereo ADSR uses 25 ms attack, 180 ms decay, 0.88 sustain, and
  320 ms release; and
- one reported, explicit fixed presentation gain per complete composite feeds
  the static -0.3 dBFS hard ceiling.

The source renderer, pitch schedules, deterministic seeds, local safety fades,
and stereo construction are unchanged. There is no new mechanism, compressor,
automatic limiter, soft saturator, maximizer, time-varying gain,
post-render normalization, or separately audible layer envelope.

The 48 kHz fixed presentation gains are:

| composite | linear gain |
| --- | ---: |
| twelve-layer reference | 33.50 |
| three singles | 32.75 |
| three chords | 44.75 |
| three stereo progressions | 51.50 |
| three mono progressions | 54.25 |
| Cross anchor | 34.75 |
| Spectral anchor | 33.25 |
| Dual anchor | 34.50 |

All eight WAVs pass the predeclared gate. Active RMS is -10.053 to
-10.003 dBFS and ceiling contact is 0-0.297%. Tonal coverage is 100%, spectral
flatness is 0.138-0.440, and no output is classified as noise-like. Mono loss
is 0-0.956 dB, correlation is 0.616-1.000, absolute DC is below 0.000134, and
maximum adjacent-sample jump is below 0.188.

The eight-times post-sum residuals are -4.692 dB for the reference, -3.471 dB
for singles, -4.355 dB for chords, -3.713 dB for stereo progressions,
-3.802 dB for mono progressions, -4.996 dB for the Cross anchor, -6.675 dB
for the Spectral anchor, and -2.553 dB for the Dual anchor. These are
engineering limitations, not alias-clean or sample-rate-invariant claims.

Two fresh release batches are byte-identical except
`workstation-cost.txt`. Their deterministic aggregate SHA-256 is
`e3d88ce822abd66885d79289d1fad56bd8e3e2b171cb8435864666ec949c3e2c`.
The only ignored artifact directory is
`artifacts/coherent-hybrid-composites/`; it contains exactly one orientation
reference and seven complete composites. No solo, long-delay, mono diagnostic,
sweep, failed, or noise-rejected WAV is present.

Human listening rejected this gate because the files presented as steady
tones. The nominal common envelope used a 0.88 sustain across long sources, so
its attack variations did not provide the requested audible envelope
comparison. The generated batch was deleted. No sound was selected, mapped to
controls, preserved as a tracked preset, or integrated into `Engine`.

## 2026-07-24 monophonic envelope audition

This focused audition retains only the exact single-note D2 render from each
of the Cross, Spectral, and Dual families. The sources begin at fixed
`0/2/5 ms` offsets and are equal-power trimmed, summed as one stereo voice,
and then controlled by one common post-sum envelope. There are no chord,
progression, solo, or separately entering layer files.

The three exact 2.4-second profiles are:

| file | attack | decay | sustain | release |
| --- | ---: | ---: | ---: | ---: |
| `01_monophonic_attack_006ms.wav` | 6 ms | 220 ms | 0.58 | 500 ms |
| `02_monophonic_attack_035ms.wav` | 35 ms | 220 ms | 0.58 | 500 ms |
| `03_monophonic_attack_140ms.wav` | 140 ms | 220 ms | 0.58 | 500 ms |

Release begins at 1.9 seconds for every file. One shared fixed presentation
gain of 51.5 feeds a static -0.3 dBFS ceiling. There is no analysis-dependent
per-profile normalization, compressor, automatic limiter, or soft saturator.

The gate measures whole-file total RMS and refuses to write a WAV below
-14 dBFS. The three retained profiles measure:

| attack | total RMS | ceiling contact | correlation | mono loss |
| ---: | ---: | ---: | ---: | ---: |
| 6 ms | -10.123 dBFS | 0.033% | 0.885 | 0.466 dB |
| 35 ms | -10.076 dBFS | 0.118% | 0.885 | 0.468 dB |
| 140 ms | -10.039 dBFS | 0.275% | 0.883 | 0.476 dB |

All three retain 100% tonal coverage, spectral flatness 0.371, no noise-like
classification, absolute DC below 0.000431, maximum adjacent-sample jump below
0.136, finite output, and valid tails. These metrics reject obvious defects;
they do not establish musical quality.

Two release generations are byte-identical except
`workstation-cost.txt`. Their deterministic aggregate SHA-256 is
`c495a4d26c572db307502b9a70f9e2593fa955050723f62c508ad451fcf13bbc`.
The ignored batch is `artifacts/monophonic-envelope-composites/`, containing
exactly the three WAVs above and their reports. These are parameter positions
within one topology for the explicitly requested envelope comparison, not a
claim of three fundamentally different listening variations. Human listening
is open; nothing is selected, mapped, or integrated.
