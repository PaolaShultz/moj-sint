# Kick Comparison With Snares Listening-Batch Design

Date: 2026-08-03

Status: approved by the user for implementation.

## Purpose

Create one disposable, engine-authentic listening batch that compares the two
new Moj Sint kick experiments with the three approved SHR Drums kick voices:

1. Moj Sint House Impact;
2. Moj Sint Long Pressure;
3. SHR Drums Big Rock (Muldjord);
4. SHR Drums Experimental Noise (Muldjord); and
5. SHR Drums Electronic House.

The batch must expose authored level differences instead of hiding them, then
provide a separately labelled level-matched reel for timbre-only comparison.
Kick-and-snare contexts must show both a controlled common-snare comparison and
each old kit's native kick/snare relationship.

Human listening decides usefulness. Measurements describe level and headroom;
they do not rank or select a kick.

## Ownership and isolation

The comparison belongs to `/home/shome/p/moj-sint` because it evaluates the
isolated Moj Sint kick candidates. Its only retained output is the ignored,
disposable directory:

`artifacts/kick-comparison-with-snares/`

The renderer may use a temporary Cargo project outside every repository to
load the existing Moj Sint WAV evidence and invoke the sibling SHR Drums
library. Temporary source and build output must be removed after successful
verification. No production Moj Sint, SHR-DAW, or SHR Drums source, preset,
catalog, runtime configuration, JACK/ALSA state, or installed file changes.

The authoritative factory packages are read directly from:

- `/home/shome/p/shr-daw/kits/big-rock-muldjord.shrkit`;
- `/home/shome/p/shr-daw/kits/experimental-noise-muldjord.shrkit`; and
- `/home/shome/p/shr-daw/kits/electronic-house.shrkit`.

## Considered approaches

### Selected: raw solos, controlled common-snare patterns, native patterns, and two reels

This is the smallest batch that answers both requested questions: how the five
kicks compare at their authored output levels, and how their kick/snare balance
behaves. One common Electronic House snare isolates kick differences; native
old-kit patterns retain each package's internal bus interaction. Raw and
level-matched reels keep level judgment separate from timbre judgment.

### Rejected: solos plus native old-kit patterns only

This is shorter, but the different native snares make it difficult to tell
whether a balance impression comes from the kick or the snare. It also gives
the two new kicks no equivalent musical context.

### Rejected: level-matched batch only

Matching every file would make timbre comparison easy but erase the authored
peak, RMS, crest, and mix-headroom differences the user explicitly wants to
inspect.

## Source and rendering contract

All output is deterministic 48 kHz stereo float32 WAV. Every trigger uses MIDI
velocity 110. SHR Drums uses `ProjectKey::default()` and
`KitTuning::default()`. Note 36 is the kick and note 38 is the snare in all
three factory packages.

Moj Sint source audio comes from the already verified files in
`artifacts/two-clean-house-kicks/`. Their samples are copied without gain,
normalization, resampling, or processing; shorter material is zero-padded when
needed. SHR Drums sources are rendered through `load_package` and `DrumEngine`,
not extracted from package WAVs. This preserves modeled components, velocity
response, round robin, envelopes, kit-bus processing, and authored gain.

Raw output has no added EQ, compression, saturation, reverb, delay, limiter,
normalization, or clipping. Common-snare files render an Electronic House
snare-only engine and linearly sum that exact source with each kick presentation.
Even the Electronic House common-snare file uses separate kick-only and
snare-only renders so all five controlled comparisons share the same topology.

Native old-kit files trigger kick and snare inside one instance of that kit's
engine. They intentionally retain package-level bus interaction and are not
directly interchangeable with the controlled common-snare files.

## Timing

Solo files contain one kick at 250 ms and last three seconds. The existing Moj
Sint solo sources are placed at that same trigger time; SHR Drums sources are
triggered directly at that frame. No tail may be truncated: if a source is
still active at three seconds, the solo duration expands for every source to
the shortest common duration that includes the complete longest tail plus
250 ms of verified silence.

Pattern files contain four bars of 4/4 at 124 BPM. Kicks trigger on every
quarter note. Snares trigger on beats 2 and 4. A two-second tail follows the
last bar; the renderer may lengthen that common tail if any engine remains
active. All five common-snare patterns have exactly the same frame count. The
three native old-kit patterns use that same frame count.

## Exact artifact inventory

The directory contains exactly these 15 WAV files:

1. `01_house-impact_solo_raw.wav`;
2. `02_long-pressure_solo_raw.wav`;
3. `03_big-rock-muldjord_solo_raw.wav`;
4. `04_experimental-noise-muldjord_solo_raw.wav`;
5. `05_electronic-house_solo_raw.wav`;
6. `11_house-impact_common-snare_124bpm_raw.wav`;
7. `12_long-pressure_common-snare_124bpm_raw.wav`;
8. `13_big-rock-muldjord_common-snare_124bpm_raw.wav`;
9. `14_experimental-noise-muldjord_common-snare_124bpm_raw.wav`;
10. `15_electronic-house_common-snare_124bpm_raw.wav`;
11. `21_big-rock-muldjord_native-kick-snare_124bpm_raw.wav`;
12. `22_experimental-noise-muldjord_native-kick-snare_124bpm_raw.wav`;
13. `23_electronic-house_native-kick-snare_124bpm_raw.wav`;
14. `31_all-kicks_raw-level-reel.wav`; and
15. `32_all-kicks_level-matched-reel.wav`.

The raw reel presents the five raw solos in inventory order, separated by one
second of digital silence, without gain changes. The matched reel applies one
declared static gain per source so all five solo peaks equal the quietest raw
solo peak. It never boosts the reference source, cannot create clipping, and
is labelled as unsuitable for judging authored level.

Non-WAV evidence files are:

- `README.md`: listening order, exact raw/matched boundary, and interpretation;
- `manifest.tsv`: filename, source, kit, notes, velocity, BPM, gain policy, and
  frame count;
- `metrics.tsv`: peak dBFS, fixed-window RMS dBFS, crest factor, absolute mean,
  maximum adjacent-sample jump, and remaining headroom for every WAV;
- `source-metrics.tsv`: isolated kick and common-snare measurements used to
  explain each controlled mix;
- `hashes.tsv`: SHA-256 for every deterministic file except itself and
  workstation timing;
- `generation-summary.tsv`: source revisions, package manifest hashes, sample
  rate, encoding, and pass/fail gates; and
- `workstation-cost.txt`: explicitly nondeterministic elapsed generation time.

## Level and safety measurements

Metrics use decoded stereo samples. Peak is the greatest absolute sample. RMS
uses the complete non-padding presentation window declared in `manifest.tsv`.
Crest factor is peak dBFS minus RMS dBFS. Headroom is `0 dBFS - peak dBFS`.
Absolute mean and maximum adjacent-sample jump are reported independently for
left and right, with the greater absolute result retained.

Every raw solo and pattern must remain finite and strictly below absolute 1.0.
If a linearly summed common-snare file would reach or exceed 1.0, generation
must fail instead of adding a limiter or hidden attenuation. The failure report
must identify the source and measured overage so a separately approved static
presentation gain can be designed; the renderer must not choose one silently.

Round-robin selection is deterministic because every file starts from a fresh
engine with the same event order. Package sample hashes were verified before
this design and are checked again during generation.

## Verification and listening order

Before handoff, generation must prove:

- all three package manifests have the expected kit IDs;
- every referenced Muldjord WAV exists and matches its manifest SHA-256;
- all WAVs are 48 kHz stereo float32 and contain only finite samples;
- exact solo and pattern frame-count groups match;
- all raw files remain below 0 dBFS without ceiling contact;
- the matched reel's five segment peaks agree within 0.01 dB;
- two fresh generations are byte-identical except `workstation-cost.txt`;
- the final directory has exactly the declared inventory and is ignored by
  Git; and
- neither repository acquires uncommitted changes.

Listen first to files 01-05 for authored impact and decay, then 11-15 for the
controlled snare balance, then 21-23 for native-kit behavior. Use file 31 to
confirm the practical level hierarchy and file 32 only to compare timbre after
level bias is removed.

No result is promoted into Moj Sint, SHR-DAW, or a factory kit until the user
listens and explicitly selects a next action.
