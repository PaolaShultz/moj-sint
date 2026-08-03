# Two Clean House Kicks Listening-Gate Design

Date: 2026-08-03

Status: approved for specification by the user. Implementation remains gated
on review of this written specification.

## Purpose

Build a small, isolated listening gate for two separate electronic kick voices:

1. **House Impact**, a tight 200-400 ms four-on-the-floor kick that leaves room
   for a separate bass line; and
2. **Long Pressure**, a 500-900 ms bass-carrying kick with a deliberately shaped
   tail.

Both voices must sound massive through their source topology, envelope motion,
phase, and controlled resonance. They must not obtain weight by clipping,
waveshaping, saturation, limiting, compression, automatic normalization, or a
noise layer. Human listening decides whether either voice is musically useful.

## Scope and ownership

The experiment belongs to `/home/shome/p/moj-sint` and remains offline and
isolated. It may add a focused research module, an offline renderer, contract
tests, and concise durable documentation. It must not change production
`Engine`, model selection, presets, the twelve-control contract, JACK, ALSA,
SHR-DAW, factory catalogs, or Raspberry Pi claims.

Generated WAVs, settings, and measurements belong under the ignored
`artifacts/` tree. They are disposable unless the user explicitly selects a
specific result for preservation.

## Considered approaches

### Selected: one phase-modulated oscillator and one coupled resonator

House Impact uses a direct carrier/modifier oscillator while Long Pressure uses
energy transfer between two damped resonant modes. The two voices therefore
test different sound-generation hypotheses rather than different decay settings
inside one graph. This also gives the requested envelope-driven modifier a
clear causal role without making distortion responsible for the result.

### Rejected: two pitch-envelope oscillators

This is cheaper and easier to tune, but the listening comparison would likely
contain short and long versions of the same identity. It does not satisfy the
repository rule that presented variations use fundamentally different source
mechanisms.

### Rejected: two resonator variants

Two resonators could both sound weighty, but their distinction would depend too
heavily on damping, coupling, and excitation coordinates. They also share the
same stability and retrigger risks. A single resonator voice is enough for this
gate.

## Shared preparation and rendering boundary

All coefficient preparation, envelope-time conversion, phase increments, file
metadata, and analysis setup happen outside the sample loop. Rendering uses
fixed-capacity state and may not allocate or deallocate, lock, perform I/O, log,
format, panic, spawn, or perform avoidable per-sample transcendental setup.
The sine notation below describes the signal mathematically; implementation
uses a prepared fixed sine table with bounded interpolation. Long Pressure's
time-varying coefficient trajectories are likewise prepared before rendering.

Both voices are mono sources written identically to left and right. No stereo
decorrelation is part of this experiment. A note trigger initializes the
one-shot envelopes and deterministic phase/state. Repeated notes are rendered
through the same persistent voice state so the tests expose overlap and
retrigger behavior rather than concatenating independent WAVs.

Each voice has one static, declared output gain. The solo and repeated renders
use that same gain. There is no per-file normalization or hidden ceiling stage.
The retained experiment prepares its analytic trajectory and a linear 5 Hz DC
blocker outside the sample path; trigger and sampling only reset/read an index.
That precomputed representation is an audition implementation, not a settled
production memory architecture.

## Voice 1: House Impact

### Signal flow

```text
note trigger
  -> amplitude envelope A(t)
  -> shape envelope E(t)
       -> carrier pitch trajectory fc(t)
       -> modifier pitch trajectory fm(t)
       -> phase-modulation index I(t)
  -> modifier m(t) = sin(phi_m(t))
  -> carrier x(t) = A(t) * sin(phi_c(t) + I(t) * m(t))
  -> static output gain
```

The oscillator phases are accumulated from their instantaneous frequencies;
the implementation must not restart a waveform at a nonzero amplitude or
derive phase by multiplying absolute time by a changing frequency. The modifier
is audible only through its effect on carrier phase. It is not mixed as a
second layer.

### Starting coordinate and bounded tuning

- settled carrier: 52 Hz;
- initial carrier: 156 Hz;
- carrier pitch settling time: 55 ms;
- modifier/carrier frequency ratio: 2:1;
- initial phase-modulation index: 0.9 radians;
- modifier-index settling time: 45 ms;
- amplitude attack: 1 ms with a zero-amplitude start;
- nominal -60 dB amplitude-decay time: 320 ms; and
- total solo render: 1.0 second.

Engineering tuning may move the settled carrier within 48-58 Hz, the initial
carrier within 120-180 Hz, the pitch settling time within 35-75 ms, the ratio
within 1.5-2.5, the initial index within 0.4-1.2 radians, and the -60 dB decay
within 220-400 ms. Such a sweep is diagnostic only. It must choose one retained
coordinate by preferring the lowest modulation index that passes the declared
shape and level gates; sweep points are not listening variations.

### Intended identity

The first few cycles provide a compact, pitch-coherent attack rather than a
separate click. As the shape envelope falls, both pitch and harmonic complexity
settle into a clean low body. The tail ends quickly enough for a house bass
line to occupy the spaces between and beneath repeated kicks.

## Voice 2: Long Pressure

### Signal flow

```text
note trigger
  -> 1 ms finite raised-cosine excitation
  -> short-lived analytic impact mode
       -> envelope-controlled center frequency and coupling contour
  -> analytic low body mode
       -> slow pitch sigh and damped decay
  -> linear weighted sum
  -> static output gain
```

The impact-mode envelope is the modifier: it controls the causal rise into the
body mode rather than creating a third, separately presented click layer. The
retained implementation evaluates both damped swept modes in closed form after
a recursive two-pole prototype failed the sample-rate-consistency gate. The
system remains linear and time-varying. It contains no feedback nonlinearity,
rail model, clipper, saturator, or post-drive branch.

### Starting coordinate and bounded tuning

- body resonance: 58 Hz at trigger, settling to 48 Hz;
- body pitch-sigh time: 280 ms;
- nominal body -60 dB decay: 760 ms;
- impact resonance: 112 Hz at trigger, settling to 76 Hz;
- impact resonance -60 dB decay: 115 ms;
- coupling-envelope settling time: 100 ms;
- excitation length: 1 ms with finite start and end values; and
- total solo render: 1.5 seconds.

Engineering tuning may move the settled body within 44-54 Hz, body decay within
550-900 ms, impact resonance within 85-140 Hz, and coupling settling within
70-140 ms. Diagnostic sweeps choose the lowest coupling that passes the
declared causal-ablation and level gates. They are not additional listening
variations.

### Intended identity

The impact mode supplies a clean upper-body push and then transfers attention
to a centered low resonance. The slow pitch sigh and longer decay let the kick
carry bass energy, while the faster upper-mode decay prevents the complete tail
from remaining hard or clicky.

## Listening presentation

The retained artifact directory is `artifacts/two-clean-house-kicks/`. It
contains exactly four WAV files:

1. `01_house-impact_solo.wav`;
2. `02_long-pressure_solo.wav`;
3. `03_house-impact_124bpm.wav`; and
4. `04_long-pressure_124bpm.wav`.

The repeated files contain four bars of quarter-note, four-on-the-floor hits at
124 BPM. They use the same source state and gain as the solo files. They are
context presentations of the two mechanisms, not extra sound variations. The
directory also contains `README.md`, `settings.tsv`, `metrics.tsv`, and the
recorded deterministic hash. No third-party sample, loop, preset, or musical
recording is used.

Listening begins at a controlled playback level. The decision questions are:

- Does House Impact feel immediate and heavy without sounding clipped, fizzy,
  or split into click plus sub?
- Does it leave plausible room for an independent house bass line?
- Does Long Pressure carry convincing bass weight without becoming muddy,
  boomy, or obviously resonant?
- Does each repeated render remain authoritative without objectionable overlap
  or a detached attack?
- Are the two mechanisms plainly different musical tools?

## Automated rejection gates

Automated evidence rejects broken candidates; it does not select musical
quality.

Both retained voices must:

- produce only finite samples and return to exact zero after their reported
  tail plus zero padding; once all envelopes are inactive and stored energy is
  below `1e-8`, a block-boundary silence policy clears the remaining state;
- render deterministically with byte-identical WAV and report content across
  two release generations, excluding explicitly declared workstation timing;
- allocate and deallocate zero times after preparation, including trigger,
  retrigger, and block rendering;
- keep sample peak and estimated 8x true peak at or below -1.0 dBFS/dBTP, while
  keeping sample peak at or above -6.0 dBFS so weak sources are not presented;
- have zero samples at or beyond absolute amplitude 0.999 and use no hidden
  clip, limiter, compressor, saturation, or normalization stage;
- keep absolute full-file mean below `1e-5` and maximum adjacent-sample jump
  below `0.10`;
- compare the 48 kHz render with an independently generated 8x render reduced
  through the existing windowed-sinc reference-resampling method, with residual
  RMS no higher than -60 dB relative to the active reference signal;
- meet the declared -60 dB decay window and retain a monotonic overall decay
  after the initial impact window; and
- survive solo trigger, 124 BPM quarter-note retrigger, 124 BPM eighth-note
  stress, and a trigger received before the previous tail has ended.

House Impact must additionally:

- settle within the retained 48-58 Hz body range by 80 ms;
- show no phase discontinuity at the end of its pitch trajectory; and
- show at least a 6 dB first-80-ms difference in 100-800 Hz energy when the
  modifier is causally muted, while changing 180-320 ms low-body RMS by no more
  than 1 dB.

Long Pressure must additionally:

- keep every resonator pole inside the unit circle for the complete coefficient
  trajectory and show monotonically decreasing stored energy after excitation;
- settle within the retained 44-54 Hz body range by 350 ms; and
- show at least a 4 dB first-140-ms difference in 70-250 Hz energy when the
  impact-to-body coupling is causally muted, while retaining measurable
  44-54 Hz body energy from 250 ms through the declared tail.

Active-window RMS, crest factor, low-band energy, instantaneous-frequency
trajectory, and spectral centroid are reported but do not rank the voices.

## Testing and failure behavior

Development is test-first. Contract tests cover exact artifact inventory,
settings/report schema, deterministic rendering, phase and coefficient
trajectories, causal ablations, finiteness, bounds, allocations, decay,
retriggering, and the high-rate reference comparison. Any failed rejection gate
prevents the primary WAVs from being written; diagnostic outputs remain
temporary and are removed before the listening handoff.

If no coordinate within a voice's declared bounds passes, implementation stops
with that mechanism rejected. It must not widen the bounds, add distortion,
add layers, normalize the result, or preserve a partial batch without a design
revision.

## Research provenance and licensing boundary

- Kurt James Werner, Jonathan S. Abel, and Julius O. Smith III, “A
  Physically-Informed, Circuit-Bendable, Digital Model of the Roland TR-808
  Bass Drum Circuit,” *Proceedings of DAFx-14*, 2014, pp. 159-166.
  <https://pure.qub.ac.uk/files/124500900/dafx14_kurt_james_werner_a_physically_informed_ci.pdf>
  The institutional copy states © 2014 the authors. This design uses the
  paper's factual decomposition of excitation, resonant body, brief center-
  frequency motion, retriggering, and longer pitch sigh. It copies no code,
  prose, equations, parameter table, or figure.
- Roland Corporation, *TR-808 Service Notes*, first edition, June 1981.
  <https://manuals.plus/m/471e79efe21a3f44a7bfcc01e2ac2145869d12240070c1644e57f5db236b7cd5.pdf>
  This proprietary service document is used only for the factual observation
  that the bass-drum circuit briefly raises resonant frequency before a fixed
  decaying body. No schematic, component layout, prose, or circuit clone is
  reproduced.
- John M. Chowning, “The Synthesis of Complex Audio Spectra by Means of
  Frequency Modulation,” *Journal of the Audio Engineering Society* 21(7),
  1973, pp. 526-534.
  <https://charlesames.net/pdf/JohnChowning/frequency-modulation.pdf>
  Publisher copyright should be presumed. The experiment independently uses
  only the mathematical fact that a time-varying modulation index produces a
  dynamic spectrum. It copies no implementation, patch, prose, or figure.

The two voices are original Moj Sint experiments, not emulations or compatible
presets for an 808, 909, or another commercial instrument.

## Completion boundary

Implementation is complete only when the focused and normal repository checks
required by `AGENTS.md` pass, the four-file listening batch is regenerated from
a release build, the artifact/report inventory is exact, and the user receives
the two solo and two repeated renders with an honest engineering summary.
Neither sound is accepted for production until the user listens and explicitly
selects it.
