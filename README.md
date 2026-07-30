# Moj Sint

Moj Sint is a headless Rust Model D instrument and a distinct managed backend
for SHR-DAW. Its library keeps DSP independent of JACK, ALSA, files, processes,
and clocks; the binary provides strict preset validation, deterministic offline
rendering, and the live JACK/ALSA host.

The production control surface has twelve stable absolute controls:
`EVOLVE`, `SHAPE`, `COLOR`, `EDGE`, `COUPLE`, `MOTION`, `DEPTH`, `SPACE`,
`ATTACK`, `DECAY`, `SUSTAIN`, and `RELEASE`. The idealized Model D path is the
reference preset's default baseline. Moving the first eight controls opens all
modeled Model D experiment mechanisms for evaluation; none of those mechanisms
has been rejected.

## Usage

```sh
# Validate or render without JACK
cargo run -- validate presets/reference.mojsint
cargo run -- render presets/reference.mojsint output.wav --note 60 --seconds 1

# Live host: joins an existing JACK server but never starts or reconfigures it
cargo run -- --client-name shs-moj-sint --preset presets/reference.mojsint
```

The live process publishes one ALSA Sequencer input named `input` and exactly
two JACK outputs named `out_l` and `out_r`. It does not connect them to physical
or other JACK ports.

## Local installation

```sh
cargo install --path . --locked
install -Dm644 presets/reference.mojsint \
  "$HOME/.local/share/moj-sint/presets/reference.mojsint"
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
