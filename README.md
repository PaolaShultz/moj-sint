# Moj Sint

Moj Sint is a low-level, headless Rust synthesizer intended to run as a
distinct external instrument managed by SHR-DAW. The primary deployment target
is a Raspberry Pi 5; development is also supported on x86_64 Linux.

The repository currently contains a pure DSP engine, deterministic offline
renderer, scalar bandlimited oscillator experiments, and analysis tooling. It
does not yet contain the live JACK/ALSA host.

The current branch is experimental, not a usable instrument release. An
integrated-wavetable saw/square route and a shared-phase fundamental/third-
harmonic selector are implemented. Automated checks pass, but the user rejected
the selector's listening results as too close to plain sine material and not
usefully distinctive. That route remains only as a documented negative research
result and test baseline; it is not an accepted factory sound or final macro
mapping.

No Raspberry Pi performance, safe-polyphony, latency, or sound-quality claim is
made yet. Generated listening artifacts are intentionally not stored in the
repository after review.

## Development

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run -- validate presets/reference.mojsint
cargo run -- render presets/reference.mojsint output.wav --note 60 --seconds 1
```

See `docs/ARCHITECTURE.md`, `docs/HOST_CONTRACT.md`, and `docs/PORTABILITY.md`.
