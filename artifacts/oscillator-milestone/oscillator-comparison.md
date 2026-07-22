# Oscillator comparison

`alias_error_db` is residual energy relative to a finite band-limited Fourier reference after DC removal and one fitted scalar gain. It conservatively includes aliasing plus amplitude and phase error; more-negative values are better.

| Method | Mean error (dB) | Worst error (dB) | Selected |
| --- | ---: | ---: | :---: |
| polyblep | -25.188 | -18.325 | no |
| integrated_wavetable | -25.586 | -18.325 | yes |

Selection rule: lower worst-case residual wins; values within 0.1 dB are treated as tied and use lower aggregate residual, then the smaller/no-table method. Evidence selects **integrated_wavetable**.

Raw conditions: 48 kHz, 32 coherent periods at the nearest integer-sample period to MIDI notes 36/60/84/96, and shape 0.00/0.50/1.00. See `oscillator-comparison.tsv`.
