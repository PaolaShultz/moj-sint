# Acid-chain synthesis research

Status: original research record, 2026-09-04. The owner requested live
integration on September 5; current Engine and schema contracts are in
[Preset and control schema](PRESET_SCHEMA.md). Listening acceptance remains
open for the three independently selectable topologies.

## Question

Which functional ideas behind the Behringer TD-3-SB are worth carrying into
an original Moj Sint instrument without reproducing its circuit, panel,
sequencer, presets, or product identity?

The answer is the causal chain and its performance coupling, not an exact
component model:

```text
band-limited reverse-ramp or pulse source
    -> resonant low-pass cascade
    -> amplitude contour
    -> optional post-chain nonlinear colour

velocity/pressure -> filter motion + loudness + short-lived memory
overlapping note  -> pitch slew without contour retrigger
```

## Primary product facts

Behringer describes the TD-3 as a monophonic analog instrument with one VCO,
one low-pass VCF, and one envelope. Its oscillator selects reverse-sawtooth or
pulse. The tone section exposes cutoff, resonance, envelope depth, decay, and
accent. A switchable distortion stage follows the synth path and exposes
drive, tone, and level. The internal sequencer stores notes/rests and per-step
accent and slide information.

Sources:

- Behringer, **TD-3 product page**, current product documentation,
  <https://www.behringer.com/en/products/0718-ABP>. The manufacturer calls the
  source a transistor-shaped reverse-saw/pulse VCO and the filter a 24 dB/oct
  low-pass, and places distortion at the last stage of the analog path.
- Music Tribe / Behringer, **TD-3 Quick Start Guide**, document P0ECM,
  <https://mediadl.musictribe.com/media/PLM/data/docs/P0ECM/TD-3_QSG-WW.pdf>.
  The guide is the authority for the control functions, monophonic/one-VCO/
  one-VCF/one-envelope inventory, connections, and sequencer operations.
- B&H Photo, an authorized dealer, **Behringer TD-3-SB product listing**,
  <https://www.bhphotovideo.com/c/product/1821005-REG/behringer_td_3_sb_analog_bass_line_synthesizer.html/specs>.
  Used only to resolve the exact suffix: `SB` is the Strawberry colourway.
- Roland Corporation, **TB-303 Owner's Manual**,
  <https://cdn.roland.com/assets/media/pdf/TB-303_OM.pdf>. This earlier source
  documents the same broad saw/square, cutoff, resonance, envelope-modulation,
  decay, accent, and step-performance vocabulary that informed the TD-3.
- Roland Corporation, **TB-303 Service Notes**, 19 February 1982,
  <https://usermanual.wiki/Document/rolandtb303servicenotesrvgm.1195579485.pdf>.
  Consulted only for functional block relationships and the qualitative
  interaction between envelope amount, cutoff bias, and exponential control;
  no schematic, component value, adjustment, or circuit expression is copied.

`SB` is the Strawberry/red colour variant (`SR` is silver), not a different
synthesis architecture. The design target is therefore the ordinary TD-3
signal behavior rather than a separate SB engine.

## What actually creates the articulation

The six tone controls do not explain the whole result. Roland's current
sequencer documentation says accent raises both output level and filter cutoff,
while slide ties notes and moves continuously between pitches. Robin Whittle's
long-running TB-303 modification research gives the more useful systems view:

- a volume contour and a main/filter contour have different jobs;
- an accented event makes the filter contour short, adds loudness, and charges
  a separate sweep path;
- that sweep charge may remain between close accents, so repeated pressure has
  memory rather than behaving as independent velocity scaling;
- a slide holds the gate while pitch slews into the next note rather than
  producing an ordinary retrigger.

Sources:

- Roland Corporation, **Mastering the TB-303 Sequencer in Roland Cloud**,
  <https://articles.roland.com/mastering-the-tb-303-sequencer-in-roland-cloud/>.
- Robin Whittle, **Investigating some Unique aspects of the TB-303's sound**,
  1 June 1999, <https://www.firstpr.com.au/rwi/dfish/303-unique.html>.
- Robin Whittle, **Investigating the Slide function of the TB-303**, 31 May
  1999 with 2012 caveat, <https://www.firstpr.com.au/rwi/dfish/303-slide.html>.

Whittle's pages are copyrighted explanatory material. They support only the
qualitative state relationships above. No prose, diagram, values, code, or
exact response curve is reused.

## Filter interpretation

Marketing and historical descriptions alternate between 18 dB/oct and
24 dB/oct. The useful engineering conclusion is not to choose a slogan: the
hardware contains a four-energy-storage-stage unbuffered ladder whose
pass-band and resonance behavior does not reduce to four identical buffered
one-poles. Tim Stinchcombe's analysis supports four poles and explains why its
measured/asymptotic descriptions can be confused.

Source:

- Tim Stinchcombe, **Analysis of the Moog Transistor Ladder and Derivative
  Filters**, <https://www.timstinchcombe.co.uk/synth/Moog_ladder_tf.pdf>.
  Copyright retained by the author. Used only to reject the simplistic
  “three-pole because 18 dB” assumption; no transfer function, circuit values,
  diagram, code, or fitted constants are copied.

## Original Moj Sint proposal

Working name: **Pressure Chain**. This name and its controls must not imply
TD-3/TB-303 compatibility.

The experiment retains the recognizable source categories and serial causality
but deliberately changes the mechanism:

1. An independently written PolyBLEP oscillator continuously blends a falling
   ramp and variable-width pulse. It does not reproduce transistor waveforms.
2. Four causal smoothed low-pass cells with explicitly delayed feedback form
   the main cascade. Each stage has a small bounded odd nonlinearity, but there
   is no circuit-derived solver or copied ladder transfer function.
3. Three genuinely different routings test the idea rather than three knob
   positions:
   - **Deep Cascade:** all four cells in series, followed by colour.
   - **Body Tap:** a protected two-cell low body is recombined with the
     four-cell edge path before colour.
   - **Cross Feed:** differentiated stage-two energy is fed, with a bounded
     one-sample delay, into the cascade input and post-colour path.
4. Velocity charges an original **pressure memory**. The state decays between
   notes and jointly changes cutoff motion, pulse asymmetry, and post-chain
   colour. This preserves the principle of coupled, stateful articulation but
   not the reference response.
5. A slide event slews pitch without restarting the filter or amplitude
   contour. Moj Sint's normal ADSR remains the four-control loudness surface;
   filter sweep decay is an independent timbral control.

The exact eight timbral roles are `SOURCE`, `SHAPE`, `CUTOFF`, `RESONANCE`,
`SWEEP`, `DECAY`, `PRESSURE`, and `BITE`, followed by the settled amp ADSR.
There is no sequencer, pattern memory, tune control, distortion switch, hidden
page, or extra physical control in the experiment.

## Clean-room and product boundary

- Do not inspect or copy TD-3 firmware, patterns, presets, PCB artwork, or
  third-party emulation source.
- Do not encode service-manual component values or calibration points.
- Do not use `TD-3`, `TB-303`, `303`, `acid clone`, or Behringer/Roland marks as
  the model or preset identity.
- The cited facts guide an independently authored topology only. The output is
  not expected or tested to null against either hardware product.
- Initial evidence stays in an ignored `artifacts/pressure-chain-listening/`
  directory. Production Engine, preset schema, factory catalog, and SHR-DAW
  integration require a separate listening decision.

## Admission gate

Before listening, every topology must be deterministic, finite, bounded,
resettable, and allocation-free through note/control/sample operations. Every
timbral role must create a measurable change. Oscillator/nonlinear behavior
must have an explicit 48 kHz versus high-rate residual measurement. The three
listening files must exercise different routings, not parameter sweeps passed
off as different instruments.
