# Portability and Deployment

## Current evidence

The DSP source is scalar Rust with `f32` audio. The live binary dynamically
loads `libjack.so.0` and links the ALSA Sequencer adapter through `alsa`.
Native Raspberry Pi 5 callback simulation now establishes an engine-budget
provisional cap of eight tested voices at 48 kHz/64 frames. A corrected
ten-minute eight-voice rapid-control soak used 3.992% of the period at p99.9
and 9.751% at maximum, with zero deadline misses, finite output, flat RSS, and
no new throttling. This does not establish whole-system SHR/JACK polyphony:
The live host is now implemented but has not been exercised against connected
JACK or measured inside SHR's audio graph.

The native x86_64 Ubuntu test/build remains the other development check.
An `aarch64-unknown-linux-gnu` compile check proves Rust-level portability only;
it does not prove that the binary links, runs, meets deadlines, or sounds right
on Raspberry Pi OS.

Rustup installs only the target standard library. Cross-linking later live-host
code also requires an AArch64 linker/sysroot and target ALSA/JACK libraries; see
the [rustup cross-compilation guide](https://rust-lang.github.io/rustup/cross-compilation.html).

## Raspberry Pi 5 acceptance

On the actual 2 GB Pi 5 with 64-bit Raspberry Pi OS and the production audio
interface:

1. install stable Rust and `libasound2-dev` (plus JACK headers only if the final
   boundary links rather than dynamically loads);
2. build and test natively in debug and release;
3. validate the JACK client, exactly two ports, ALSA MIDI identity, shutdown,
   reconnect, and xrun reporting;
4. measure callback time distribution, xruns, end-to-end MIDI/audio latency,
   CPU, memory, temperature/throttling, and sustained behavior;
5. repeat across intended sample rates, buffer sizes, voice loads, presets, and
   polyphonic phrases; and
6. perform listening tests before making sound-quality or control-usefulness
   claims.

The official Pi 5 brief identifies a 64-bit Cortex-A76 platform, but hardware
specifications are not performance evidence:
<https://datasheets.raspberrypi.com/rpi5/raspberry-pi-5-product-brief.pdf>.

## Dependencies

Current foundation: the exact Rust 1.97.1 toolchain selected by
`rust-toolchain.toml`, with rustfmt and Clippy installed user-locally through
rustup. Building the ALSA adapter requires:

```sh
sudo apt update
sudo apt install libasound2-dev
```

JACK headers are not required because the host loads `libjack.so.0` dynamically.
JACK/PipeWire services and hardware remain outside installation scope; the host
will fail rather than start or reconfigure a server.

Install the validated binary and all 13 allowlisted public factory presets for
SHR-DAW with the commands in the repository README. The binary belongs in the user's
Cargo `bin` directory; `.mojsint` files belong under the configured data root,
never under ignored repository `artifacts/`.
