# Moj Sint

Moj Sint is a headless Rust Model D instrument and a distinct managed backend
for SHR-DAW. Its library keeps DSP independent of JACK, ALSA, files, processes,
and clocks; the binary provides strict preset validation, deterministic offline
rendering, and the live JACK/ALSA host.

The production control surface has twelve stable absolute controls:
`EVOLVE`, `SHAPE`, `COLOR`, `EDGE`, `COUPLE`, `MOTION`, `DEPTH`, `SPACE`,
`ATTACK`, `DECAY`, `SUSTAIN`, and `RELEASE`. Seven factory starting points
preserve the exact authored bass, lead, filter-articulation, and matched
diagnostic configurations. Moving the first eight controls opens the modeled
mechanisms from any starting point; none has been rejected.

## Usage

```sh
# Validate or render without JACK
cargo run -- validate presets/reference.mojsint
cargo run -- render presets/reference.mojsint output.wav --note 60 --seconds 1

# Live host: joins an existing JACK server but never starts or reconfigures it
cargo run -- --client-name shs-moj-sint --preset presets/reference.mojsint
```

The factory catalog is ordered as Full Bass, Full Lead, Full Filter
Articulation, Matched Idealized, Matched Linear Mixer, Matched Linear Ladder,
and Matched No Drift or Feedback. These are editable live starting points, not
seven claims of separate synthesis families.

The live process publishes one ALSA Sequencer input named `input` and exactly
two JACK outputs named `out_l` and `out_r`. It does not connect them to physical
or other JACK ports.

## Local installation

```sh
cargo install --path . --locked
install -d "$HOME/.local/share/moj-sint/presets"
while IFS= read -r preset; do
  install -m644 "presets/$preset" "$HOME/.local/share/moj-sint/presets/"
done < presets/cleared-presets.txt
```

SHR-DAW's default configuration includes that user preset root. Presets added
there are user data; they are not copied back into either repository.

## Development

```sh
cargo fmt --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --all-targets
```

See [architecture](docs/ARCHITECTURE.md), the
[live-host contract](docs/HOST_CONTRACT.md), the
[preset/control schema](docs/PRESET_SCHEMA.md), and
[portability notes](docs/PORTABILITY.md).

The cleared public installation boundary and dependency licence review are in
[THIRD_PARTY.md](THIRD_PARTY.md). Only the files named by
`presets/cleared-presets.txt` are factory-installable.

Moj Sint source and its project-authored factory presets are available under
the [MIT License](LICENSE).
