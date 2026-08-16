# Bass Matrix

Bass Matrix is the fifth Moj Sint model and the second new instrument in the
2026-08-16 SHR integration. It is intentionally unlike the Swarm Machine: one
voice starts with a mono-compatible bass core, then adds controlled upper
harmonics instead of building a wide oscillator population.

## Signal flow

```text
note + pitch punch
  ├─ main sine ───────────────┐
  ├─ phase-locked f/2 sub ────┴─ linear body filter ───────┐
  ├─ phase modulation ─ growl ─┐                            │
  └─ inharmonic ring branch ───┴ drive (2 substeps) ─ filter
                                      └ bounded feedback ───┤
                         DC blocker ─ output guard ─ volume ─┴─ stereo
```

The main and sub phases reset together at Note On. The true half-frequency sub
shares the outer amplitude envelope and never enters the nonlinear branch.
Only generated upper material reaches asymmetric shaping, filter emphasis,
feedback, and stereo side motion. That separation is the mechanism intended to
retain bass weight while the upper branch becomes dirty or unstable.

The nonlinear branch evaluates the current sample and one midpoint substep.
This is a fixed two-step selective oversampling measure, not a claim of
alias-free waveshaping. Note-tracked filter coefficients, pitch-envelope decay,
and the DC blocker are prepared outside the per-sample path. Every feedback
state is finite-guarded and bounded; the final static guard is ±0.94.

## Physical controls

All values are continuous `0..=1` and move through Moj Sint's 10 ms smoothing
path. Position 5 is independent instrument volume.

| Position | Label | Minimum | Middle | Maximum |
| ---: | --- | --- | --- | --- |
| 1 | `BODY` | focused fundamental | main/sub blend | strongest f/2 weight |
| 2 | `GROWL` | clean body | audible PM rasp | deep phase-modulated growl |
| 3 | `METAL` | harmonic | ring/asymmetry color | inharmonic metallic branch |
| 4 | `PUNCH` | stable onset | short pitch hit | two-octave transient dive |
| 5 | `VOLUME` | silence | half linear gain | authored full level |
| 6 | `DRIVE` | nearly linear grit branch | clear saturation | hard bounded shaping |
| 7 | `FILTER` | dark upper branch | open body/grit blend | bright resonant emphasis |
| 8 | `UNSTABLE` | locked and centered | light jitter/feedback/side | bounded unstable motion |
| 9–12 | ADSR | short/low envelope endpoints | useful middle ranges | long/high endpoints |

The factory `Transformer` start is deliberately conservative. From that one
loaded sound, lower `GROWL`, `METAL`, `DRIVE`, and `UNSTABLE` for clean bass;
raise `BODY` for sub weight; raise `GROWL` and `DRIVE` for dirt; raise `METAL`
for ring-like upper partials; use `PUNCH` for attack; and combine `FILTER` with
`UNSTABLE` only after setting a safe listening level.

## Research and provenance

The design was independently authored from functional DSP ideas, not copied
source:

- Vadim Zavalishin, [*The Art of VA Filter Design*, revision 2.1.2](https://www.native-instruments.com/fileadmin/ni_media/downloads/pdf/VAFilterDesign_2.1.2.pdf), informed the explicit state, saturation, and feedback-filter reasoning.
- Vesa Välimäki and Antti Huovilainen, [“Antialiasing Oscillators in Subtractive Synthesis”](https://research.aalto.fi/en/publications/antialiasing-oscillators-in-subtractive-synthesis/), and Juhan Nam et al., [“Efficient Antialiasing Oscillator Algorithms Using Low-Order Fractional Delay Filters”](https://mac.kaist.ac.kr/pubs/jnam-taslp2010.pdf), support treating discontinuous, sync, and modulated oscillator aliasing as a design constraint rather than assuming naïve waveforms are safe.
- Julian D. Parker, Vadim Zavalishin, and Efflam Le Bivic, [“Reducing the Aliasing of Nonlinear Waveshaping Using Continuous-Time Convolution”](https://www.dafx.de/paper-archive/details/vem_XXF5qBbfiWOH2RVVAA), supports the warning that nonlinear and feedback waveshaping creates above-Nyquist products and that low-order oversampling can be useful.
- [Surge XT](https://github.com/surge-synthesizer/surge) and [VCV Fundamental's VCF](https://github.com/VCVRack/Fundamental/blob/v2/src/VCF.cpp) were inspected only as GPL-3.0 open-source comparisons for established synth structure, sub-oscillator practice, bounded filter outputs, and project-level safety habits. No GPL code, constants, text, or presets were copied into the MIT implementation.

The complete applied-source register and claim boundaries remain in
[Bass Impact Research](BASS_IMPACT_RESEARCH.md).

## Automated evidence and open verdict

`tests/bass_matrix_contract.rs` renders clean, sub-heavy, driven, growling, and
extreme configurations from the same deterministic voice. It records RMS,
peak, adjacent-sample difference RMS, and a sample-bit hash; it also requires
material residual differences between neighboring settings. The same suite
checks every timbre control at minimum/middle/maximum, deterministic finite
output, the ±0.94 guard, stereo activity, reset silence, and allocation-free
rapid movement.

The pinned 48 kHz, MIDI-note-36, 24,000-frame run records:

| Setting | RMS | Peak | Adjacent-difference RMS | FNV sample hash |
| --- | ---: | ---: | ---: | --- |
| clean | 0.30868754 | 0.47272879 | 0.00263003 | `269eedafaf888f4d` |
| sub-heavy | 0.31709681 | 0.52742177 | 0.00171504 | `b4c31205560e48d1` |
| driven | 0.32188900 | 0.51177996 | 0.00306734 | `ca6f2298541c8a10` |
| growling | 0.21131847 | 0.50728464 | 0.00883832 | `bbb5e94e3bf04031` |
| extreme | 0.18610712 | 0.46742240 | 0.00880144 | `acca5115f36b9baf` |

These measurements prove deterministic and materially different output. They
do not establish that the sounds are musical, pleasant, heavy, or acceptable.
That verdict belongs to listening in SHR-DAW. Native Raspberry Pi 5 callback
headroom also remains a hardware measurement, not a workstation inference.
