# Moj Sint

Moj Sint is a low-level, headless Rust synthesizer intended to run as a
distinct external instrument managed by SHR-DAW. The primary deployment target
is a Raspberry Pi 5; development is also supported on x86_64 Linux.

This initial foundation contains a pure DSP engine and deterministic offline
renderer. It does not yet contain the live JACK/ALSA host.

## Development

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -- validate presets/reference.mojsint
cargo run -- render presets/reference.mojsint output.wav --note 60 --seconds 1
```

See `docs/ARCHITECTURE.md`, `docs/HOST_CONTRACT.md`, and `docs/PORTABILITY.md`.
