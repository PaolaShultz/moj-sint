# Bass Impact Research

Date: 2026-07-23

## Purpose and limits

This document investigates a central Moj Sint goal: a bass identity that feels
immediately forceful and desirable—the user's metaphor is “bass that cuts
kidneys” and makes a listener want the machine. The metaphor is a sound-design
brief, not a physiological claim. Here, **impact** means a controlled
combination of:

- a clearly timed onset and short-term loudness rise;
- retained pitch and low-band energy;
- useful peak-to-RMS structure rather than indiscriminate limiting;
- pitch-coherent harmonic evidence on bandwidth-limited loudspeakers;
- bounded roughness, beating, or nonlinear products that add threat without
  destroying the note;
- resonant energy whose decay is deliberate rather than muddy or unstable; and
- stereo motion that survives mono and does not remove the low-frequency
  anchor.

No waveform can guarantee acoustic pressure, bodily vibration, or safe
listening. Those outcomes also depend on the D/A converter, amplifier,
headphones or loudspeakers, room, listener position, playback gain, exposure
duration, and the listener. Moj Sint can create waveform cues and preserve
headroom; it cannot turn a small transducer into a subwoofer or determine sound
pressure level at the ear.

This is research guidance for the disposable final hybrid sound lab. It does
not accept a voice, assign a stable macro, alter `Engine`, establish Raspberry
Pi cost, or replace human listening.

## Findings

### 1. Weight, punch, pressure, tactile impression, and menace are separable

“Weight” is not one measurable quantity. For this lab it should be treated as
the joint observation of low-band RMS/energy, stable pitch evidence, and a
decay long enough to carry energy without masking the next event. “Punch”
should be reserved for onset-weighted behavior: Fenton and Lee's listening
tests found that a model combining onset detection, frequency-band information,
and transient loudness tracked subjective punch better than simple crest-factor
alternatives. This supports measuring a time-frequency onset, not maximizing
one broadband peak.

Acoustic “pressure” and tactile sensation cannot be inferred from a normalized
WAV. Takahashi's 20–50 Hz experiment found vibratory-sensation thresholds
5–17 dB SPL above hearing thresholds in its tested normal-hearing subjects.
That finding does not define a production target; it demonstrates that actual
vibration is level- and reproduction-dependent. The sound lab should therefore
ask whether a waveform *suggests* pressure at a fixed, controlled monitor
level, while refusing to claim that it produces bodily sensation.

“Menace” is also not a universal acoustic variable. Arnal, Kleinschmidt,
Spinelli, Giraud, and Mégevand found enhanced aversion and salience for rapidly
amplitude-modulated sounds in a 30–150 Hz roughness range. This supports a
bounded roughness experiment, not the claim that roughness is always desirable
or that it alone means menace. Moj Sint should keep roughness on a controlled
branch, compare it at matched loudness, and reject fatigue or pitch loss in
listening.

ISO 226:2023 further prevents a simplistic “more sub equals more weight”
conclusion: equal perceived loudness for pure tones depends strongly on
frequency and listening conditions. A normalized digital level is neither
equal loudness nor acoustic SPL.

**Design consequence:** record onset-band energy, active RMS, crest factor,
decay timing, pitch evidence, and roughness-related modulation separately.
Never combine them into an undocumented “impact score.”

### 2. Missing-fundamental reinforcement improves translation, not displacement

A harmonic complex can retain the pitch associated with a fundamental even
when the physical fundamental is weak or absent. Virtual-bass systems exploit
this by adding pitch-related harmonics above a small loudspeaker's low-frequency
limit. Bai and Lin demonstrated a phase-vocoder system for this use case and
validated it with objective and MUSHRA-style subjective tests.

That evidence does **not** mean that any distortion creates useful bass.
Translation requires:

- harmonics derived from each note's own pitch;
- controlled harmonic order and magnitude;
- continuity through pitch changes;
- no assumption that generated upper harmonics provide physical sub-bass; and
- testing on both bandwidth-limited and full-range reproduction.

For Moj Sint, a per-note harmonic spine is preferable to post-mix
rectification. With a three-note chord, a shared nonlinear stage creates sum
and difference products between notes. Those products need not belong to any
note's harmonic series and can obscure pitch. A per-voice generator produces
only products related to that voice until the linear mix.

An actual subharmonic is different from a missing-fundamental cue. A divided
phase or register event can create a real component at `f0/2`; upper harmonics
can imply `f0` on a small speaker. The former consumes low-band headroom and
may change the perceived register. The latter aids translation but does not
replace acoustic low-frequency capability.

**Design consequence:** preserve a real, centered fundamental when the
reproduction system can carry it, add a sparse per-voice harmonic spine for
translation, and measure the two paths independently.

### 3. Attack, phase, crest factor, and envelope curvature determine whether
energy arrives as punch or mud

Fenton and Lee's result makes onset-weighted loudness the most defensible
perceptual lead. A bass onset should therefore have at least two coordinated
time scales:

1. a short, band-limited attack cue that establishes timing; and
2. a slower low-band rise that avoids a discontinuity and leaves headroom for
   the attack.

The exact curves remain an audition variable. A concave or two-stage amplitude
rise can let the attack band speak before the body reaches steady state; a
single instantaneous broadband step risks clicks, excess peak, and an onset
that is bright but not heavy. A long low-band attack may sound detached or
late. A long release or high-Q decay can overlap the next event and become mud.

Phase matters even when the magnitude spectrum is unchanged. Schroeder showed
that harmonic phase can be chosen to create low-peak-factor signals for a
fixed power spectrum. The converse is important here: zero- or onset-aligned
partials can create a large periodic peak without adding spectral energy.
Therefore:

- phase alignment is a crest-shaping control, not free loudness;
- low crest factor can create sustained density but erase transient contrast;
- high crest factor can create a useful strike or simply waste headroom; and
- chord voices must not all reset to the same phase, because their aligned
  peaks can produce avoidable mix overload.

**Design consequence:** report peak, RMS, crest factor, true peak, attack-band
energy, time to peak, and time to a chosen fraction of steady low-band energy.
Use deterministic but different initial phase/seed per chord voice.

### 4. Nonlinearity must be judged by products, placement, DC, and aliasing

Saturation, clipping, folding, rectification, and nonlinear filters are not
interchangeable:

- symmetric odd saturation mainly adds odd harmonics to a symmetric
  single-tone input;
- asymmetric drive and half-wave behavior add even as well as odd components
  and can shift the mean;
- full-wave rectification has a strong even-harmonic structure and removes the
  input sign;
- hard clipping sharply limits peaks but creates a wide harmonic series;
- folding reverses slope after thresholds and can create repeated corners; and
- nonlinear feedback changes both spectrum and state evolution.

Every one can alias. Parker, Zavalishin, and Le Bivic show why sampled
memoryless waveshaping generates components above Nyquist that fold into the
audio band. Bilbao and colleagues' antiderivative method and Albertini,
Bernardini, and Sarti's nonlinear wave-digital-filter work provide candidate
antialiasing directions, but the appropriate method is topology-specific.
A high-rate reference comparison remains required for every proposed branch.

Every one can also create intermodulation in a chord. Leinonen and Otala found
that no single traditional distortion measurement exposed all tested
mechanisms and recommended complementary dynamic and two-tone methods. For the
lab this means:

- single-tone harmonic measurements are insufficient;
- note-pair and complete three-note probes are required;
- per-voice nonlinearity is the default;
- any shared post-mix nonlinearity needs a direct comparison against separate
  per-voice processing; and
- generated content below, between, and above chord notes must be reported,
  not hidden inside total harmonic distortion.

DC must be measured before and after any blocker. A DC blocker can protect the
output while leaving an asymmetric internal bias that changes a feedback
system's operating point. Internal state means and saturation dwell time are
therefore useful diagnostics for asymmetric and feedback paths.

**Design consequence:** audition several nonlinear *topologies*, not several
drive settings of one topology. Keep nonlinear branches band-limited,
per-voice, gain-compensated, and bypass-comparable.

### 5. Resonators create body only when modes, damping, and excitation are
controlled

Karplus and Strong establish the basic causal lesson: a short excitation and a
feedback delay can generate a sustained pitch-bearing sound. Smith and
Rocchesso show that feedback-delay networks can be described through a
feedback matrix and give conditions for a lossless network. Chowdhury gives
explicit stability conditions for nonlinear biquad structures rather than
assuming that inserting a soft clip makes a recursive filter safe.

For bass design, lossless is a mathematical boundary, not the musical target.
A practical body needs:

- note-related delay or modal frequencies;
- damping below sustained runaway;
- deliberately unequal modes so the whole network does not reinforce one
  frequency;
- finite excitation energy;
- bounded cross-coupling;
- a DC policy inside as well as after the loop;
- coefficient preparation outside sampling; and
- reset behavior that cannot inherit arbitrary energy from the previous note.

One-note ringing occurs when the same fixed resonance dominates different MIDI
notes. Mud occurs when modal decay overlaps chord changes or adjacent notes.
A nonlinear loop can self-bias, change its effective decay with amplitude, or
generate aliased products even when its linearized poles look safe.

**Design consequence:** measure per-mode decay, pitch tracking, total stored
energy, impulse-response envelope, and release overlap. Reject a resonator
whose strongest mode does not follow the note or whose energy stops decaying
after excitation ends.

### 6. A complete bass voice needs roles, not indiscriminate layers

The useful layer model is causal:

```text
per-note phase/state
  -> centered fundamental/body
  -> pitch-coherent upper harmonic spine
  -> short high-frequency onset cue
  -> optional subharmonic or resonant energy
  -> topology-owned stereo structure
  -> conservative linear voice mix
```

The layers must not all run at full level. Their roles are:

- **fundamental:** pitch and real low-band energy;
- **subharmonic:** optional lower-register weight, enabled only when headroom
  and chord behavior remain acceptable;
- **low-mid texture:** small-speaker pitch/identity and controlled roughness;
- **high attack:** onset timing, then rapid decay so it does not become hiss;
- **gate:** suppresses noise, folding residue, or resonator spill without
  chopping the body; and
- **integer/register event:** sparse deterministic excitation, modulation,
  addressing, or coupling-state change—not a standalone swarm by default.

Sparse carry/borrow events are especially suitable as exciters because their
event timing can be pitch-related while their energy is separately bounded.
They are dangerous as direct discontinuities; their impulse magnitude,
minimum spacing, and band-limited response need explicit limits.

**Design consequence:** render diagnostic stems during development, but judge
the complete topology in the listening gate. A layer that is inaudible,
redundant, or only increases RMS should be removed.

### 7. Chords change the engineering problem

Three notes introduce six recurring risks:

1. **Masking:** close partials share auditory bandwidth and obscure one another.
   Moore and Glasberg's harmonic-complex masking work supports measuring
   detectability around the partial structure instead of assuming that every
   generated harmonic remains perceptually available.
2. **Beating and roughness:** unresolved close components create envelope
   fluctuation. Roughness can add controlled tension but can also blur chord
   identity. The complete chord, not isolated voices, is the relevant test.
3. **Intermodulation:** a shared nonlinearity creates products from every note
   pair. Separate per-voice nonlinear paths avoid those cross-note products
   until the linear mix.
4. **Phase cancellation and reinforcement:** deterministic initial phases can
   either remove or exaggerate short-term energy. Use different per-note
   phases/seeds and test more than one deterministic seed set.
5. **Headroom:** summing three bounded voices is not itself bounded to one
   voice's peak. Reserve fixed mix headroom before any loudness matching.
6. **Note retention:** a loud root does not prove that the third and fifth
   remain audible. Project all three target frequencies and their useful
   harmonic neighborhoods in each sustained segment.

Roughness is not synonymous with musical instability in every culture or
context. The PLOS ONE cross-cultural study by Milne, Smit, Sarvasy, and Dean
supports an association between chord roughness and perceived stability, but
it does not authorize a universal “menacing chord” rule. Moj Sint should
measure roughness-related energy and ask the listener whether it provides
desired tension without losing the three notes.

**Design consequence:** every nonlinear or resonant topology must pass the
single note, held chord, and progression. No mechanism passes on a single bass
note alone.

### 8. Stereo bass should preserve a mid anchor while allowing designed side
energy

The familiar instruction “make all bass mono” is too broad. Pulkki and
Karjalainen found low-frequency interaural time-difference cues to be
consistent in stereo localization, and Cabrera, Ferguson, and Subkey reported
left-right discrimination in their 25–100 Hz low-frequency experiment.
Low-frequency stereo can therefore be perceptually meaningful.

That does not make arbitrary antiphase bass safe. Boers showed that conflicting
time and level cues can produce a vague virtual image. Cross-coupled combs,
asymmetric filters, and micro-delays can all generate large side energy or
comb cancellation when folded to mono.

The robust strategy is not a universal fixed crossover. It is:

- retain a centered, correlated direct/body component;
- place most deliberate asymmetry in resonant, low-mid, and attack paths;
- keep cross-feedback and micro-delay gains bounded;
- measure low-band mid and side separately;
- listen to the actual mono fold, not only a correlation number; and
- test headphones and loudspeakers because their crosstalk differs.

**Design consequence:** require non-identical channels and nonzero side energy,
but reject silent or spectrally hollow mono, accidental polarity inversion,
and low-band side dominance that removes the pitch anchor.

### 9. Related movement should not collapse into one synchronized pump

The auditory system is sensitive to modulation rate. Dau, Kollmeier, and
Kohlrausch's modulation-filterbank model accounts for modulation detection and
masking with rate-selective processing. It supports measuring modulation
spectra instead of describing all motion as “an LFO.”

There is no cited result here that says irrational rate ratios are inherently
musical. The bounded Moj Sint hypothesis is narrower:

- share a causal event or envelope when layers should feel related;
- use different prepared rates and phases for spectral traversal, resonator
  motion, and stereo delay;
- avoid exact common periods inside the 10–16 second audition window unless
  an audible cycle is intended;
- smooth register-derived pulses before using them as continuous modulation;
  and
- inspect the modulation spectrum for one dominant common rate and harmonics.

**Design consequence:** record declared rates and their pairwise ratios, render
long enough to observe the slowest motion twice, and reject obvious global
pumping unless the listener explicitly prefers it.

### 10. Engineering evidence is descriptive and rejective

The lab should produce measurements for each voice, condition, channel, stem,
and mono fold:

| Evidence | Required observation |
| --- | --- |
| Finite/bounded | No NaN/Inf; explicit peak bound before loudness matching |
| DC | Mean per channel, per internal nonlinear/feedback state where practical, and mono fold |
| Level | Sample peak, ITU-R BS.1770 true peak, active RMS, crest factor, and matched listening gain |
| Transient | Onset time, time to peak, attack-band energy, low-band rise time, adjacent-sample jump |
| Decay | Time to fixed energy fractions, residual energy before next event, per-mode decay where observable |
| Spectrum | Band energy, partial projections, spectral centroid/rolloff as descriptions, not quality scores |
| Pitch | Fitted fundamental and harmonic support for every single note |
| Chord | Projection of all three note fundamentals and selected harmonics in every segment |
| Products | Two-tone and three-tone intermodulation/difference/sum products; non-harmonic residual |
| Aliasing/error | Eight-times-rate conservative residual for low/middle/high notes and strong nonlinear settings |
| Stereo | L/R difference, correlation, mid/side energy by band, interchannel delay/phase behavior |
| Mono | Fold RMS/peak, note retention, low-band loss, spectral delta, and actual diagnostic WAV |
| Determinism/cost | Reset/render hashes, allocation checks, and scalar workstation timing clearly labeled |

ITU-R BS.1770-5 defines authoritative programme-loudness and true-peak
algorithms. These are useful engineering references, but neither LUFS nor
digital true peak reports acoustic SPL at the listener.

Starting rejection thresholds should be preregistered in the implementation
plan as **lab bounds**, not psychoacoustic laws. The existing Moj Sint
conventions provide reasonable starting points:

- fail any non-finite sample or any pre-normalization peak beyond the declared
  ceiling;
- fail sample-path or three-voice-ensemble allocation;
- fail a sustained absolute DC mean above the existing lab's documented
  tolerance, and inspect lower internal-state means rather than relying only on
  the output blocker;
- fail a target note that loses the existing 30 dB pitch-retention test;
- fail a chord/progression segment in which any target-note projection is
  absent under the declared projection method;
- fail a mono fold that is silent, polarity-inverted, loses a target note, or
  exceeds the declared low-band loss relative to the stereo mid;
- fail feedback energy that grows after excitation or does not cross a declared
  decay threshold before the next event;
- fail integer event paths that lock, repeat an unintended short cycle, or
  materially change pitch relationship with sample rate without disclosure;
  and
- report rather than hide every severe high-rate residual.

Crest factor, roughness, correlation, and side/mid values should initially be
reported as ranges, not pass/fail quality scores. Thresholds become meaningful
only after a small set of A/B renders establishes which range the user wants.

### 11. Safe listening and product boundaries

WHO and ITU-T H.870 use exposure—level over time—as the safety quantity. The
adult reference allowance is equivalent to 80 dBA for 40 hours per week, with
a more conservative 75 dBA mode. These are device/exposure guidelines, not a
target for producing or auditioning Moj Sint and not a guarantee of safety for
professional equipment, which H.870 excludes from scope.

For the lab:

- loudness-match files so “better” cannot mean “louder”;
- begin listening at a comfortable low level;
- do not raise playback gain merely to obtain tactile sensation;
- take breaks and count exposure from all sources;
- preserve conservative digital headroom and true-peak measurement; and
- state that the playback chain and listener determine SPL.

The desired waveform should remain compelling at controlled levels through
timing, spectrum, motion, and decay. If its identity appears only when played
loud enough to create physical vibration, the waveform-design experiment has
failed.

## Guidance for the three complete voices

### Cross-Coupled Machine Voice

Make attack timing and controlled internal opposition its identity:

- use a phase-coherent impact spine with a short upper-band onset and slower
  centered body;
- let one fixed-width state machine modulate the nonlinear PM index and emit
  sparse carry events into a damped low resonator;
- keep drive per voice and split-band so chords do not enter one shared
  nonlinear element;
- cross-couple only damped resonator/low-mid state, not the unprotected
  fundamental;
- retain a centered direct fundamental and put most stereo difference above
  the deepest band; and
- use separate prepared rates for register pulse smoothing, micro-delay, and
  vibrato.

The voice should be rejected if the register mechanism is inaudible when
removed, if it becomes rhythmic clicking, if the low resonator rings at one
fixed pitch, or if mono removes the push/answer identity.

### Spectral Shadow Voice

Make small-speaker translation and alignment/separation its identity:

- keep the authored spectral frames pitch-normalized and amplitude-normalized;
- derive a sparse upper-harmonic spine from the same per-note phase;
- treat the lower shadow as a real divided-phase/register relationship, not an
  arbitrary detached sine;
- use an all-pass or short dispersive path to change phase/crest interaction
  without pretending it adds spectral energy;
- place asymmetric nonlinear filtering per voice and high-pass its generated
  side content before the stereo sum; and
- keep spectral traversal, address perturbation, and stereo motion at related
  but non-synchronous rates.

The voice should be rejected if it becomes an organ, if the shadow consumes
headroom without improving low-level/full-range preference, if three chord
notes merge into one spectral smear, or if the mono fold loses the moving
spectral identity.

### Dual Resonant Body

Make controlled stored energy and two-body interaction its identity:

- coordinate deterministic noise, a quiet tonal carrier, and sparse
  carry/borrow events as a finite excitation packet;
- give the two bodies different mode sets and damping rather than different EQ
  on the same comb;
- keep one pitch-locked low mode centered enough to protect mono and use the
  dispersed upper body for more side energy;
- place bounded nonlinearity inside each body's loop only with explicit
  stability, DC, energy, and alias checks;
- cross-feed a strictly limited fraction of damped state; and
- measure whether each body decays and retunes at every chord transition.

The voice should be rejected if either body continues growing after excitation,
if both converge to the same mode, if the progression leaves old chord energy
masking the new chord, or if the result is merely a generic pluck.

## Five bounded bass mechanisms

These are distinct causal experiments. Parameter positions within one
mechanism are engineering sweeps, not additional listening variations.

### Mechanism 1: Phase-coherent impact spine

**Causal signal flow**

```text
per-note phase/state
  + finite onset event
  -> 2-5 ms band-limited attack cue
  -> curved 10-40 ms low/body rise
  -> steady centered fundamental + sparse 2nd/3rd/5th harmonic spine
  -> independent short attack/body decays
  -> conservative voice gain
```

The exact time ranges are starting experiment bounds, not perceptual
standards. Harmonic phases are prepared so the onset can move between a
crest-concentrated strike and a denser low-crest body without changing the
declared magnitude spectrum.

**Expected perceptual identity:** immediate timing followed by a compact,
pitch-solid body; “punch” should remain at matched long-term loudness rather
than coming from a higher file level.

**Single-note and chord behavior:** one note should show an earlier upper-band
cue and a slightly later low-band body without sounding split. Chord voices use
different deterministic phase/seed states so their attacks do not create one
avoidable aligned peak. Every note retains its own harmonic spine.

**Stereo/mono strategy:** the fundamental and first body harmonics remain
centered. Only a small upper attack/resonant return may differ between channels.
Mono should retain timing, pitch, and most body energy.

**Engineering risks:** click from discontinuous onset; excessive true peak;
attack detached from body; low crest factor erasing punch; high-frequency
alias from abrupt gating; aligned chord peaks; attack energy masking other
voices.

**Automated rejection tests:**

- finite, bounded, allocation-free samples;
- maximum adjacent-sample jump and true peak within declared limits;
- report time to attack peak and low-band rise; reject inversion of the
  intended order;
- compare matched-RMS crest factor across prepared phase conditions;
- require target-note projections in single and chord renders;
- render at eight times rate to expose onset/nonlinear residual; and
- require mono target-note and transient retention.

**Human listening questions:**

- At fixed low playback level, does the note arrive decisively?
- Is the short cue fused with the body or heard as a separate click?
- Does the chord remain three notes rather than one large transient?
- Is the denser phase condition heavier, or merely flatter?
- Does the mono fold keep the same timing and authority?

**Planned voice:** primary for **Cross-Coupled Machine Voice**; a reduced
version can excite **Dual Resonant Body**.

### Mechanism 2: Per-voice split-band asymmetric translator

**Causal signal flow**

```text
per-note source
  -> linear low/high split
  -> low band: dry anchor + bounded odd saturation
  -> high/low-mid band: biased or asymmetric soft drive
  -> generated-component extraction
  -> internal DC control + output DC blocker
  -> pitch-tracked band limit
  -> linear recombination before three-voice mix
```

The first comparison should use genuinely different transfer topologies:
bounded odd saturation, asymmetric soft drive, and one folded/rectified
candidate. Drive settings inside a topology are sweeps only.

**Expected perceptual identity:** a low-mid “teeth” layer that makes pitch and
threat legible on small speakers while the low anchor stays calm. The desired
result is not generic full-band distortion.

**Single-note and chord behavior:** nonlinear processing is per voice, so its
products remain related to that note. The chord is a linear sum of processed
voices. A deliberate shared post-mix comparison may be rendered as engineering
evidence but should be expected to create more cross-note IMD.

**Stereo/mono strategy:** keep the low anchor mid. Use small L/R differences in
the generated low-mid branch through different prepared filter trajectories,
not opposite polarity. Mono retains the dry anchor and sums the character
branch without a notch.

**Engineering risks:** DC and internal bias; even-order buildup; fold/clipping
aliasing; chord IMD; loudness bias; high-note fizz; low-note mud; generated
branch cancelling the fundamental; output blocker hiding biased loop state.

**Automated rejection tests:**

- single-tone harmonic table for low/middle/high notes;
- two-note and three-note sum/difference-product table;
- direct per-voice versus shared-post-mix IMD comparison;
- DC at the nonlinearity, after the branch filter, and at output;
- high-rate residual at moderate and strongest retained drive;
- bypass identity and gain-matched branch comparisons;
- chord-note retention and no unintended low difference tone above the
  preregistered relative limit; and
- mono spectral delta and target-note retention.

**Human listening questions:**

- Does it add a recognizable machine identity or only “distortion”?
- Is the bass easier to recognize on a bandwidth-limited speaker at the same
  loudness?
- Does menace increase without fatigue or fizz?
- Do minor chords remain intelligible?
- Is the asymmetric option worth its DC and headroom cost?

**Planned voice:** primary for **Cross-Coupled Machine Voice**; a narrower,
frame-aware version for **Spectral Shadow Voice**.

### Mechanism 3: Shadow/subharmonic braid with virtual-bass spine

**Causal signal flow**

```text
per-note phase + authored spectral frame
  -> centered real fundamental/body
  -> divided phase/register state -> optional f0/2 shadow
  -> same-note phase -> selected upper harmonics implying f0
  -> bounded all-pass/dispersive phase rotation of shadow
  -> level/energy compensation
  -> per-channel nonlinear filter pair
```

The real `f0/2` shadow and the upper virtual-bass spine are separately
controllable experimental paths. They must not be described as the same
psychoacoustic mechanism.

**Expected perceptual identity:** a transforming object with a lower shadow
that alternately locks to and pulls away from its visible spectrum, while upper
harmonics keep the note legible on smaller reproduction.

**Single-note and chord behavior:** each voice derives its own shadow and
harmonics. For chords, the subharmonic path begins at a much lower level or is
disabled in one comparison because three simultaneous octave-down components
can consume headroom and obscure roots. The upper spine remains per voice.

**Stereo/mono strategy:** fundamental and lowest shadow energy retain a strong
mid component. Phase rotation and asymmetric filters create side energy mainly
in upper shadow harmonics and low mids. Mono folding must not cancel the real
fundamental or the virtual-bass spine.

**Engineering risks:** detached octave-down sine; excessive low-band RMS;
ambiguous chord register; phase cancellation; all-pass crest spikes; organ-like
static frames; filter-state cross-coupling instability; virtual bass confused
with actual sub capability.

**Automated rejection tests:**

- separate energy reports for `f0`, `f0/2`, and selected upper harmonics;
- chord headroom and each-note projection with shadow off/on;
- small-speaker diagnostic high-pass renders as analysis only;
- crest-factor and true-peak delta from all-pass rotation;
- L/R and mono partial-retention tables;
- nonlinear-filter stability, DC, and alias/error checks; and
- traversal normalization so frame changes do not become loudness pumping.

**Human listening questions:**

- Is the lower shadow fused with the object or heard as another oscillator?
- Does the harmonic spine improve identity on a small speaker without making
  full-range playback nasal?
- Is `f0/2` useful in a three-note chord or only for single notes?
- Does phase motion feel like shape change rather than chorus?
- Does mono preserve the same pitch center?

**Planned voice:** **Spectral Shadow Voice**.

### Mechanism 4: Energy-normalized cross-coupled dual body

**Causal signal flow**

```text
finite tonal/noise/register excitation
  -> Body A: pitch-locked low resonator -> damped nonlinear feedback
  -> Body B: dispersed upper modes -> soft nonlinear gate
  -> bounded A-to-B and B-to-A damped state transfer
  -> per-body energy monitor/compensation
  -> centered direct excitation + stereo body returns
```

The two coupling coefficients are bounded independently. Damping occurs before
cross-feed so energy cannot circulate through an undamped bypass. Any
normalization uses a slow prepared control law and cannot allocate, inspect
future samples, or conceal instability.

**Expected perceptual identity:** a struck/bowed hybrid with two connected
physical-seeming bodies; a short event becomes stored, moving, pitch-bearing
energy rather than a static oscillator plus reverb.

**Single-note and chord behavior:** each note owns both bodies. Mode frequencies
track that note with limited, declared dispersion. At chord changes, outgoing
energy uses a bounded release/crossfade; it cannot remain at the old chord
indefinitely.

**Stereo/mono strategy:** Body A contributes more correlated low energy; Body B
contributes more dispersed side energy. Cross-feed is not antiphase widening.
A centered direct excitation protects timing and mono pitch.

**Engineering risks:** runaway feedback; nonlinear self-bias; one-note ringing;
mode collision; old-chord masking; denormals; excessive crest from coincident
modes; interpolation noise; the two bodies converging to the same state;
aliasing inside nonlinear loops.

**Automated rejection tests:**

- impulse/finite-excitation energy must decay after input stops;
- bound maximum state and output for the full coupling grid;
- per-mode decay and note-tracking reports at low/middle/high notes;
- internal state mean and output DC;
- reset and chord-transition residual-energy checks;
- chord-note projection before and after transitions;
- high-rate residual for each nonlinear loop;
- distinct Body A/Body B impulse-response and spectral evidence; and
- allocation-free one- and three-voice sample paths.

**Human listening questions:**

- Does it feel like stored energy in two bodies or like a pluck with effects?
- Is the sustain forceful without becoming a fixed howl?
- Can all three chord notes be followed through the decay?
- Does energy migration feel internal rather than ping-pong?
- Does mono keep the low body and enough of the second body's identity?

**Planned voice:** **Dual Resonant Body**.

### Mechanism 5: Mid-anchored decorrelated pressure field

**Causal signal flow**

```text
centered direct/body component
  -> two unequal short comb/filter paths
  -> strictly damped cross-coupling
  -> asymmetric micro-delays with smoothed, distinct rates
  -> frequency-shaped side return
  -> L/R sum with fixed mid reserve
```

This is a topology-owned stereo mechanism, not a shared chorus after the voice.
The “pressure field” name describes the intended image, not acoustic pressure
at the listener.

**Expected perceptual identity:** a stable central mass with internal L/R
answering and slow shape change; spacious enough to notice, compact enough to
retain force.

**Single-note and chord behavior:** each voice owns its stereo state and
deterministic seed. Chord voices use different initial states without random
run-to-run behavior. Slow motion rates differ across resonator, micro-delay,
and timbral paths so the chord does not pump as one block.

**Stereo/mono strategy:** explicitly reserve mid energy in the fundamental/body
band. Shape the side return upward or make it mode-selective rather than
forcing all content below a fixed universal crossover to mono. Measure
low-band side/mid energy and mono survival directly.

**Engineering risks:** comb notches in mono; conflicting ITD/ILD cues; vague or
unstable image; static widening; side-dominant low band; modulation pitch
wobble; feedback growth; rates synchronizing within the audition; headphone
success but speaker failure.

**Automated rejection tests:**

- non-identical L/R and nonzero side energy over windows covering two cycles of
  the slowest motion;
- correlation, full-band and low-band mid/side energy;
- mono RMS/peak, target-note retention, and spectral loss;
- cross-feedback energy-decay bound;
- delay, slew, and maximum adjacent-sample-jump limits;
- modulation-spectrum report and common-period check over the render;
- deterministic alternative seed/phase set; and
- true-peak checks in L, R, mid, side, and mono.

**Human listening questions:**

- Is there a centered mass, or does the bass become wide and weak?
- Does movement feel like the machine's body rather than chorus?
- Are headphones and speakers both convincing?
- Does the mono fold still make the voice desirable?
- Does a chord remain spatially complex without becoming vague?

**Planned voice:** primary stereo structure for **Cross-Coupled Machine
Voice**, with a slower two-body variant for **Dual Resonant Body**.

## Recommended experiment order

1. Implement and validate Mechanism 1's transient/body timing as a shared
   primitive, but tune it separately inside each complete topology.
2. Give Cross-Coupled Machine Voice Mechanisms 1, 2, and 5 with one small
   register machine acting as modulator/event source.
3. Give Spectral Shadow Voice Mechanism 3 plus only the narrow nonlinear branch
   from Mechanism 2.
4. Give Dual Resonant Body Mechanism 4 plus the reduced onset exciter from
   Mechanism 1; use Mechanism 5 only through its own two bodies, not a pasted
   shared effect.
5. Render complete single notes, chords, progressions, and mono diagnostics.
   Diagnostic topology/bypass comparisons may support engineering decisions,
   but they are not additional musical variations.

The three complete voices must remain structurally distinct. Reusing a small
attack, DC-block, interpolation, or analysis primitive does not make their
sound-generation mechanisms equivalent.

## Unresolved design decisions

- Exact onset/body envelope curves and timing ranges; evidence supports their
  importance but does not select one Moj Sint curve.
- Whether any real `f0/2` energy belongs in three-note Spectral Shadow chords,
  or only in single-note/low-root conditions.
- Which distinct nonlinear topology—odd saturation, asymmetric soft drive, or
  folding/rectification—earns the Cross-Coupled Machine character branch after
  alias, DC, IMD, and listening comparisons.
- The prepared damping/coupling bounds that give Dual Resonant Body useful
  sustain without one-note ringing; these need implementation evidence.
- How much low-band side energy survives the headphone, speaker, and mono
  listening gate. No universal “mono below X Hz” rule is adopted.
- Whether roughness is a desirable identity component for this instrument or
  merely fatiguing. Automated salience evidence cannot answer that musical
  question.
- Preliminary numeric rejection thresholds for crest factor, chord IMD,
  mono-fold loss, and side/mid energy. They should be preregistered after
  baseline renders, then held fixed for the candidate comparison.

## Source and licensing register

Only factual claims, equations, and general topologies may guide an independent
Moj Sint implementation. No source code, stimuli, presets, samples, prose,
tables, or figures from these works are imported.

### Perception, punch, roughness, and safety

- **Steven Fenton and Hyunkook Lee**, “A Perceptual Model of Punch Based on
  Weighted Transient Loudness,” *Journal of the Audio Engineering Society*
  67(6), pp. 429–439, 2019, DOI
  [10.17743/jaes.2019.0017](https://doi.org/10.17743/jaes.2019.0017);
  [University of Huddersfield record](https://pure.hud.ac.uk/en/publications/a-perceptual-model-of-punch-based-on-weighted-transient-loudness/).
  **Supported claim:** listening-derived weighting of onset time and frequency
  components plus transient loudness correlated strongly with subjective punch
  and outperformed the compared simpler models. **License:** AES publication
  copyright applies; cite the result and independently define Moj Sint
  measurements.
- **Yukio Takahashi**, “Vibratory Sensation Induced by Low-Frequency Noise: A
  Pilot Study on the Threshold Level,” *Journal of Low Frequency Noise,
  Vibration and Active Control* 28(4), pp. 245–253, 2009,
  [DOI 10.1260/0263-0923.28.4.245](https://doi.org/10.1260/0263-0923.28.4.245).
  **Supported claim:** in the tested 20–50 Hz conditions, vibratory-sensation
  thresholds for normal-hearing participants were 5–17 dB SPL above hearing
  thresholds. **License:** SAGE publication copyright applies; the result
  limits claims and no stimuli or figures are reused.
- **Luc H. Arnal, Andreas Kleinschmidt, Laurent Spinelli, Anne-Lise Giraud, and
  Pierre Mégevand**,
  “The Rough Sound of Salience Enhances Aversion through Neural
  Synchronisation,” *Nature Communications* 10, 3671, 2019,
  [DOI 10.1038/s41467-019-11626-7](https://doi.org/10.1038/s41467-019-11626-7);
  [author manuscript](https://access.archive-ouverte.unige.ch/access/metadata/a9b22334-e773-49d4-a6f5-2197e953bd1f/download).
  **Supported claim:** repetitive acoustic transients in the tested 30–150 Hz
  roughness range enhanced salience/aversion relative to other modulation
  rates. **License:** the open Nature Communications article is CC BY 4.0;
  this document still paraphrases only the bounded result.
- **Torsten Dau, Birger Kollmeier, and Armin Kohlrausch**, “Modeling Auditory
  Processing of Amplitude Modulation. I. Detection and Masking with
  Narrow-Band Carriers,” *Journal of the Acoustical Society of America*
  102(5), pp. 2892–2905, 1997, DOI
  [10.1121/1.420344](https://doi.org/10.1121/1.420344);
  [institutional manuscript](https://pure.tue.nl/ws/files/1464621/621984.pdf).
  **Supported claim:** a modulation-filterbank model accounts for modulation
  detection/masking data with rate-selective processing. **License:** ASA
  publication rights apply; use the scientific result, not copied model code,
  prose, or figures.
- **ISO**, *ISO 226:2023, Acoustics—Normal Equal-Loudness-Level Contours*,
  third edition, March 2023,
  [official record](https://www.iso.org/standard/83117.html).
  **Supported claim:** perceived equal loudness of pure tones requires
  frequency- and condition-dependent SPL combinations; digital amplitude is
  not a loudness or SPL equivalence. **License:** ISO standard copyright
  applies; no normative tables or figures are reproduced.
- **World Health Organization and International Telecommunication Union**,
  *ITU-T H.870 (03/2022), Guidelines for Safe Listening Devices/Systems*,
  [official recommendation](https://www.itu.int/rec/T-REC-H.870/en) and
  [WHO overview](https://www.who.int/publications/i/item/9789241515276).
  **Supported claim:** hearing-risk guidance is exposure-based; the adult
  reference allowance corresponds to 80 dBA for 40 hours per week and the more
  conservative mode to 75 dBA for 40 hours. The recommendation excludes
  professional audio equipment from its formal scope. **License:** WHO/ITU
  publication terms apply; quote no normative text beyond short factual
  identifiers and do not present the allowance as a Moj Sint target.

### Pitch translation, phase, masking, and harmony

- **Mingsian R. Bai and Wan-Chi Lin**, “Synthesis and Implementation of Virtual
  Bass System with a Phase-Vocoder Approach,” *Journal of the Audio Engineering
  Society* 54(11), pp. 1077–1091, 2006,
  [AES record](https://secure.aes.org/forum/pubs/journal/?elib=13888).
  **Supported claim:** pitch-related upper harmonics can improve bass
  impression on small loudspeakers where direct low-frequency boosting is
  constrained; the authors evaluated their system objectively and
  subjectively. **License:** AES publication copyright applies; no algorithm
  source, stimuli, prose, or figures are copied.
- **M. R. Schroeder**, “Synthesis of Low-Peak-Factor Signals and Binary
  Sequences with Low Autocorrelation,” *IEEE Transactions on Information
  Theory* 16(1), pp. 85–89, 1970, DOI
  [10.1109/TIT.1970.1054411](https://doi.org/10.1109/TIT.1970.1054411).
  **Supported claim:** changing component phases can substantially change peak
  factor while preserving the specified power spectrum. **License:** IEEE
  publication copyright applies; independently implement only the general
  phase/crest experiment.
- **Brian C. J. Moore and Brian R. Glasberg**, “Forward Masking Patterns for
  Harmonic Complex Tones,” *Journal of the Acoustical Society of America*
  73(5), pp. 1682–1685, 1983, DOI
  [10.1121/1.389390](https://doi.org/10.1121/1.389390).
  **Supported claim:** harmonic complexes produce frequency-structured masking
  patterns; component presence in a spectrum does not guarantee equal
  perceptual availability. **License:** ASA publication copyright applies; no
  experimental stimuli or figures are reused.
- **Andrew J. Milne, Eline A. Smit, Hannah S. Sarvasy, and Roger T. Dean**,
  “Evidence for a Universal Association of Auditory Roughness with Musical
  Stability,” *PLOS ONE* 18(9), e0291642, 2023, DOI
  [10.1371/journal.pone.0291642](https://doi.org/10.1371/journal.pone.0291642).
  **Supported claim:** across the tested groups, chord roughness was associated
  with judgments of musical stability/finishedness. It does not prove a
  universal menace response. **License:** PLOS ONE articles are CC BY; reuse
  would require attribution, but this project uses only the bounded finding.

### Nonlinearity, aliasing, and distortion evidence

- **Julian D. Parker, Vadim Zavalishin, and Efflam Le Bivic**, “Reducing the
  Aliasing of Nonlinear Waveshaping Using Continuous-Time Convolution,”
  DAFx-16, Brno, 2016,
  [paper](https://www.dafx.de/paper-archive/2016/dafxpapers/20-DAFx-16_paper_41-PN.pdf).
  **Supported claim:** sampled nonlinear waveshaping creates above-Nyquist
  components that alias, and continuous-time reconstruction/convolution can
  reduce that aliasing. **License:** DAFx publication rights apply; equations
  may guide an independent comparison, with no source or figures copied.
- **Stefan Bilbao, Fabián Esqueda, Julian D. Parker, and Vesa Välimäki**,
  “Antiderivative Antialiasing for Memoryless Nonlinearities,” *IEEE Signal
  Processing Letters* 24(7), pp. 1049–1053, 2017, DOI
  [10.1109/LSP.2017.2675541](https://doi.org/10.1109/LSP.2017.2675541);
  [author manuscript](https://www.research.ed.ac.uk/files/34115216/bilbao_pdf.pdf).
  **Supported claim:** antiderivative antialiasing generalizes to arbitrary
  order for memoryless nonlinearities, with numerical corner cases that must
  be handled. **License:** IEEE publication copyright applies; independently
  derive and test any implementation.
- **Davide Albertini, Alberto Bernardini, and Augusto Sarti**,
  “Antiderivative Antialiasing in Nonlinear Wave Digital Filters,” DAFx-20,
  2020,
  [paper](https://www.dafx.de/paper-archive/2020/proceedings/papers/DAFx2020_paper_35.pdf).
  **Supported claim:** nonlinear stateful/wave-digital systems also require
  explicit alias control; the paper develops an ADAA formulation for that
  setting. **License:** CC BY 3.0 as stated in the paper; Moj Sint may use the
  mathematics with attribution but imports no implementation or figures.
- **Eero Leinonen and Matti Otala**, “Correlation of Audio Distortion
  Measurements,” *Journal of the Audio Engineering Society* 26(1/2),
  pp. 12–19, 1978,
  [AES record](https://secure.aes.org/forum/pubs/journal/?elib=3296).
  **Supported claim:** individual distortion metrics had blind spots across the
  tested mechanisms; complementary dynamic and two-tone difference-frequency
  measurements were more informative than one number. **License:** AES
  publication copyright applies; no test circuits, prose, or figures are
  copied.

### Resonance and feedback

- **Kevin Karplus and Alex Strong**, “Digital Synthesis of Plucked-String and
  Drum Timbres,” *Computer Music Journal* 7(2), pp. 43–55, 1983,
  [author-hosted paper](https://users.soe.ucsc.edu/~karplus/papers/digitar.pdf).
  **Supported claim:** finite excitation, delay, averaging/filtering, and
  feedback can generate a sustained pitch-bearing tone. **License:** MIT Press
  publication copyright applies; use only independently implemented equations
  and topology.
- **Julius O. Smith and Davide Rocchesso**, “Connections between Feedback
  Delay Networks and Waveguide Networks for Digital Reverberation,”
  *Proceedings of the International Computer Music Conference*, pp. 376–379,
  1994,
  [University of Michigan record](https://quod.lib.umich.edu/i/icmc/bbp2372.1994.098?rgn=main;view=fulltext).
  **Supported claim:** an FDN is a delay set connected through a feedback
  matrix, and a lossless matrix has unit-modulus eigenvalues with independent
  eigenvectors. **License:** the repository marks the work CC
  BY-NC-ND 3.0; cite the theorem/topology and do not adapt its prose or figures.
- **Jatin Chowdhury**, “Stable Structures for Nonlinear Biquad Filters,”
  DAFx-20, 2020,
  [DAFx record](https://dafx.de/paper-archive/details/9raXmWuilIJyqjkS4A3d8w).
  **Supported claim:** nonlinear biquad structures need explicit slope/pole
  conditions for guaranteed stability; inserting a bounded nonlinearity alone
  is not a stability proof. **License:** CC BY 3.0 as stated in the paper; no
  code or figures are copied.

### Stereo and measurement

- **Ville Pulkki and Matti Karjalainen**, “Localization of Amplitude-Panned
  Virtual Sources I: Stereophonic Panning,” *Journal of the Audio Engineering
  Society* 49(9), pp. 739–752, 2001,
  [AES record](https://secure.aes.org/forum/pubs/journal/?elib=10180).
  **Supported claim:** low-frequency ITD cues were consistent in their stereo
  localization tests/model, while ITD and ILD cues can disagree across
  frequency. **License:** AES publication copyright applies; no stimuli,
  model implementation, or figures are copied.
- **Densil Cabrera, Sam Ferguson, and Alan Subkey**, “Localization and Image
  Size Effects for Low Frequency Sound,” AES 118th Convention, paper 6325,
  2005,
  [AES record](https://secure.aes.org/forum/pubs/conventions/?elib=13041).
  **Supported claim:** the tested 25–100 Hz stimuli produced left-right
  discrimination and frequency-dependent image-size behavior; low-frequency
  stereo cannot be dismissed as categorically imperceptible. **License:** AES
  publication copyright applies; no stimuli or figures are reused.
- **Paul M. Boers**, “The Influence of Antiphase Crosstalk on the Localization
  Cues in Stereo Signals,” AES 73rd Convention, paper 1967, 1983,
  [AES record](https://secure.aes.org/forum/pubs/conventions/?elib=11793).
  **Supported claim:** antiphase crosstalk can make time and level localization
  cues conflict and produce a vague virtual image; delay changes those cue
  regions. **License:** AES publication copyright applies; the result is a risk
  boundary, not a copied widening design.
- **International Telecommunication Union**, *Recommendation ITU-R
  BS.1770-5, Algorithms to Measure Audio Programme Loudness and True-Peak Audio
  Level*, November 2023,
  [official record](https://www.itu.int/rec/R-REC-BS.1770-5-202311-I/en).
  **Supported claim:** provides standardized programme-loudness and true-peak
  algorithms suitable for reporting digital programme evidence. It does not
  report acoustic SPL at the listener. **License:** ITU publication terms
  apply; use the standard algorithm through an independent implementation and
  do not reproduce normative text, tables, or figures.

## Self-review record

- “Weight,” “punch,” “pressure,” and “menace” are decomposed into observations
  and listening questions; none is presented as a physiological guarantee.
- Every perceptual claim is bounded to the cited study or standard.
- Tactile sensation is explicitly separated from normalized waveform design.
- Missing-fundamental reinforcement is separated from real subharmonic energy.
- Nonlinear character is never accepted from harmonic distortion alone; chord
  IMD, DC, aliasing, and placement are required evidence.
- Stereo guidance avoids a universal frequency cutoff and requires actual mono
  survival.
- Numeric ranges stated for envelopes are proposed experiment bounds, not
  literature-derived laws.
- No source code, presets, samples, prose, tables, or figures were copied.
