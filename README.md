# Moj Sint

Moj Sint is a headless experimental synthesizer and a distinct managed backend
for SHR-DAW. Its library keeps DSP independent of JACK, ALSA, files, processes,
and clocks; the binary provides strict preset validation, deterministic offline
rendering, and the live JACK/ALSA host.

The current experimental catalog contains six synthesis models: circuit-
informed Model D, Six-Op PM, Strange Oscillator, the typed-graph Swarm Machine,
Bass Matrix, and Dual Filter. The first five models retain twelve stable
absolute controls. Dual Filter uses 15: two cutoff/resonance/envelope-depth
blocks, continuous `STRUCTURE`, filter ADSR, and amp ADSR. Its separate synth
rotary click crossfades between INDUSTRIAL and COUNTER cores without
retriggering a held note. The 21 factory starts are playable research outcomes
and starting points, not a production-release claim.

## Usage

```sh
# Validate or render without JACK
cargo run -- validate presets/reference.mojsint
cargo run -- render presets/reference.mojsint output.wav --note 60 --seconds 1

# Live host: joins an existing JACK server but never starts or reconfigures it
cargo run -- --client-name shs-moj-sint --preset presets/reference.mojsint
```

The factory catalog contains seven Model D starts, six Six-Op PM starts, one
Strange Oscillator start, one Swarm Machine warm pad, one Bass Matrix
transformer, and five Dual Filter starts derived from the approved lead and
four bass directions. These are editable live starting points, not 21 claims
of separate synthesis families.

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
[preset/control schema](docs/PRESET_SCHEMA.md),
[Bass Matrix design and evidence](docs/BASS_MATRIX.md), and
[portability notes](docs/PORTABILITY.md). The deliberately unscheduled
[experimental direction](docs/FUTURE_DIRECTION.md) describes open model
authoring, low-code micro-machines, and AI-assisted exploration.

The cleared public installation boundary and dependency licence review are in
[THIRD_PARTY.md](THIRD_PARTY.md). Only the files named by
`presets/cleared-presets.txt` are factory-installable.

Moj Sint source and its project-authored factory presets are available under
the [MIT License](LICENSE).
