# Dependencies, licensing, and public presets

Moj Sint source and its 13 factory `.mojsint` starts are MIT licensed. The
factory starts are project-authored parameter documents; they contain no
third-party samples, recordings, factory patches, firmware, SysEx data, source
code, prose, diagrams, or artwork. `presets/cleared-presets.txt` is the sole
public installation allowlist. Ignored `artifacts/` and private user presets
are never installable content.

The production dependency graph uses permissively licensed Rust crates:

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
