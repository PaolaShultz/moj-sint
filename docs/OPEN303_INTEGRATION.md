# Open303 in Moj Sint and SHR-DAW

Open303 is the eighth Moj Sint model, with one preallocated native voice.
It reuses the [reviewed Open303 core](OPEN303_ANALYSIS.md), with the
[vendor notices and repair ledger](../vendor/open303/README.md). The earlier
[offline candidate](OPEN303_CANDIDATE.md) remains a historical baseline.
Default builds now enable `open303`; a C++17 compiler is required. Explicit
`--no-default-features` builds reject construction of this model.

Schema 10 adds `model = "open303"` and `open303_filter = "tb303"` or
`"lowpass18"`, with `voices = 1`. Existing schemas 1–9 remain readable.
Filter topology is prepared when loading the preset, never rebuilt in a
callback. Open303 retains its own accent/slide/envelope interactions.

| Position / CC | Field | Native range |
| --- | --- | --- |
| 1 / 20 | waveform | saw–square blend, 0–1 |
| 2 / 21 | cutoff | 314–2394 Hz |
| 3 / 22 | resonance | 0–100 |
| 4 / 23 | env_mod | 0–100 |
| 5 / 7 | instrument_volume | 0–1, separate post-synthesis gain |
| 6 / 25 | filter_decay | 200–2000 ms |
| 7 / 26 | accent | 0–100 |
| 8 / 27 | slide | 0–300 ms |
| 9 / 28 | normal_attack | 0.3–30 ms |
| 10 / 29 | accent_attack | 0.3–30 ms |
| 11 / 30 | accent_decay | 30–3000 ms |
| 12 / 31 | amp_decay | 16–3000 ms |

All stored macro values are normalized 0–1 and mapped linearly to these
ranges. There is no fifth timbre macro or generic ADSR. CC24 is unused;
13–15 belong to SHR AUX. Volume follows the existing 10 ms engine smoother.
Other controls use the existing 10 ms smoothers with native setter application
every 32 samples (at most 0.73 ms at 44.1 kHz); unchanged setters are skipped.
Note events retain their exact sample offsets and apply current controls.
No table construction, allocation, locking, or I/O occurs on this path.
Inherited audio-rate cutoff modulation remains audio-rate. No new oscillator
or filter algorithm is introduced by this adapter.

MIDI velocity 100–127 selects accent; the last held key wins. Every positive
live NoteOn, including a repeated held key, retriggers the filter and amp
contours. The amp attack starts at its current level, and overlap retains the
configured pitch slide. Releasing a newer key returns to the older held pitch
and accent without retriggering. Detached notes start at their target pitch
and retrigger even while an earlier release tail is sounding. The isolated
candidate's ordinary note API keeps its original non-retriggering legato policy.
All Notes Off resets to silence. Native amp gate/release behavior is preserved;
the amp-decay control does not turn the engine into a sustain/ADSR synth.
Native output gain is fixed at −18 dB; each authored start uses .8 output gain.
The counted ±.999 ceiling/fault guard remains outside the inherited DSP.

The requested factory starts are project-authored parameter documents:

- **A03 Rubber Bass:** TB_303, saw, moderate sweep and short slide.
- **A01 Accent Wire:** TB_303, brighter resonance and stronger accent sweep.
- **A02 Hollow Slide:** LP_18, square, longer decay and slide.
- **A04 Soft Pluck:** LP_18, softer resonance and short decay.

SHR discovers these through its configured Moj catalog. Project routes,
private Save/Overwrite, RESET, current-value relative controls, and the mono
`M` title marker preserve the Open303 identity and selected filter. There is
no new backend process, navigation mode, or master-encoder action.

Software validation and deployment status are recorded in HANDOFF.md. No
listening or whole-system native callback/headroom acceptance is claimed.

Validation: normal all-target/all-feature suite 359 passed, 36 intentionally
ignored; three focused live-integration tests passed in debug and release.
All eleven native controls change audio; all-key control motion is finite,
bounded and allocation-free at 44.1/48/96 kHz. Native ASan/UBSan, C++ allocation
guards, and the existing six spectral checks pass. All 28 factory presets
validate and paired one-second release renders match exactly. Formatting,
Clippy with warnings denied, release build, audit/deny, and the minimal-feature
library check pass. The subsequent authorized combined pass built all debug/release targets in
both projects. SHR passed 1,141 normal tests (14 historical tests ignored),
and its fresh release discovered all four starts in an isolated catalog.
Normal exit/reopen loads the refreshed app; no audio or JACK restart was run.
