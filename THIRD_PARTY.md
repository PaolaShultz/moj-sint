# Dependencies, licensing, and public presets

Moj Sint source and its 28 factory `.mojsint` starts are MIT licensed. The
factory starts are project-authored parameter documents; they contain no
third-party samples, recordings, factory patches, firmware, SysEx data, source
code, prose, diagrams, or artwork. `presets/cleared-presets.txt` is the sole
public installation allowlist. Ignored `artifacts/` and private user presets
are never installable content.

The default production dependency graph uses permissively licensed Rust crates:

| Crate | Purpose | Declared licence |
| --- | --- | --- |
| `alsa` | ALSA Sequencer MIDI input | MIT OR Apache-2.0 |
| `anyhow`, `thiserror` | Error boundaries | MIT OR Apache-2.0 |
| `hound` | Deterministic offline WAV writing | Apache-2.0 |
| `libc` | Dynamic JACK and Linux FFI | MIT OR Apache-2.0 |
| `serde`, `toml` | Strict preset schema | MIT OR Apache-2.0 |
| `signal-hook` | Bounded shutdown flags | MIT OR Apache-2.0 |

JACK is loaded from the system `libjack.so.0`; it is not bundled. ALSA is a
system library linked through `alsa-sys`. The locked transitive graph and the
accepted permissive licence/source policy are checked by `cargo audit` and
`cargo deny check` with `deny.toml`. Development-only render and test crates do
not contribute public audio content.

The Six-Op PM implementation was independently authored from cited mathematical
and functional facts. Its clean-room source register and limitations are in
`docs/RESEARCH.md` and `docs/HANDOFF.md`; it makes no compatibility,
affiliation, or historical-emulation claim.

The Bass Matrix implementation was also independently authored. GPL-3.0 Surge
XT and VCV Fundamental sources were inspected only as architectural comparison
material; no code, constants, presets, or text were copied. Paper/book
provenance and the exact functional claims used are recorded in
`docs/BASS_MATRIX.md` and `docs/BASS_IMPACT_RESEARCH.md`.

Pressure Chain's oscillator, filter cells, pressure memory, topology starts,
and live adapter are independently authored. Functional research provenance
and the no-clone boundary are recorded in `docs/ACID_CHAIN_RESEARCH.md`.
No third-party DSP code, patches, samples, or circuit constants are included.

The default-enabled `open303` feature imports Robin Schmidt's MIT Open303 C++
core for the monophonic Open303 model. It adds the permissive `cc` build
crate (MIT OR Apache-2.0) and a system C++ standard library requirement.
[Vendor provenance](vendor/open303/README.md) owns the exact revision/file
manifests, retained MIT notice, separately authored Takuya Ooura FFT permission,
and local repairs. Ooura's file is not relabeled MIT. The GPL JC-303 plugin
wrapper, VST SDK, presets, and samples are not imported. Cargo's license checks
cover crates; the vendored source has this separate manual review.
The [candidate contract](docs/OPEN303_CANDIDATE.md) records its scope and limits;
the subsequent [production integration](docs/OPEN303_INTEGRATION.md) adds four
project-authored parameter presets. They contain no third-party factory patches.
