# TrueSpeech Encoder

`truespeech` is a stateful Rust implementation of the TrueSpeech encoder. It provides a Rust API, a Windows DLL with the `TrueSpeech_*` C ABI, TrueSpeech WAV writers, and a separate `no_std` DSP core. Decoding and sample-rate conversion are not included.

## Supported audio and packet format

| Property | Value |
| --- | --- |
| Input | Mono signed 16-bit PCM at 8,000 samples per second |
| Frame | 240 samples (30 ms) |
| Encoded packet | 32 bytes (256 bits) |
| Version value | `0x01000000` |

Create one `TrueSpeechContext` per audio stream and reuse it across frames. The context retains predictor history. Use `reset()` to start a new independent stream.

## Rust API

When consuming a published release, add the crate to `Cargo.toml`:

```toml
[dependencies]
truespeech = "1"
```

Encode one frame into a caller-owned packet buffer:

```rust
use truespeech::{TrueSpeechContext, TRUESPEECH_FRAME_SAMPLES, TRUESPEECH_PACKET_BYTES};

let pcm = [0_i16; TRUESPEECH_FRAME_SAMPLES];
let mut encoder = TrueSpeechContext::new();
let mut packet = [0_u8; TRUESPEECH_PACKET_BYTES];
encoder.encode_frame(&pcm, &mut packet);
```

`encode_samples_into` encodes a complete PCM slice without allocating. It zero-pads a partial final frame. Its output size can be calculated with `TrueSpeechContext::encoded_packet_bytes` before allocating a buffer. `write_truespeech_wav` writes encoded packets to any `std::io::Write`; `encode_truespeech_wav` returns the complete WAV as a `Vec<u8>`.

The runnable example creates a two-second tone and writes `speech.wav`:

```powershell
cargo run --locked --release --example encode_wav -- speech.wav
```

See the [API reference](docs/api.md) for function signatures, return values, errors, ABI requirements, and packet layout.

The WAV example accepts `-h`, `-help`, and `--help` to print its usage.

## Build a Windows distribution

On Windows with the Rust MSVC toolchain installed, build and stage the x64 DLL:

```powershell
.\build-release.ps1
```

This creates `dist\windows-x64\` containing the release DLL, MSVC import library, C header, C example, API documentation, build details, and SHA-256 checksums. For 32-bit Windows, install `i686-pc-windows-msvc` and pass it using `-Target i686-pc-windows-msvc`.

Build the library for the current Rust target:

```powershell
cargo build --locked --release --lib
```

To compile the native sample with MSVC after creating the distribution:

```powershell
cl /I dist\windows-x64 examples\use_dll.c dist\windows-x64\truespeech.dll.lib
```

Keep the DLL beside the resulting executable or on `PATH` when running it.

## WAV output behavior

The WAV helpers create a TrueSpeech RIFF file containing the extended `fmt` chunk, a `fact` chunk, and encoded `data`. A partial final frame is zero-padded. The WAV writer records the codec's 60-sample delay and emits a zero frame when needed to preserve the tail. The lower-level packet APIs do not add a delayed-tail flush frame; callers that need complete WAV duration should use a WAV helper.

## Core and architecture

The standalone `tsd32-encoder-core` workspace member builds the shared DSP source with `no_std`. A WASM build can be produced with:

```powershell
rustup target add wasm32-unknown-unknown
cargo build --locked --release --manifest-path core/Cargo.toml --target wasm32-unknown-unknown
```

The [architecture guide](docs/architecture.md) explains the modules and data flow. Editable Mermaid sources for the file map, encoder algorithm, DLL lifecycle, and public and internal functions are in [docs/diagrams](docs/diagrams/README.md).

## Scope and publishing

This package encodes TrueSpeech; it does not decode it or implement the original decoder export. The manifest declares `MIT OR Apache-2.0`; add the corresponding license texts and confirm the source provenance grants those redistribution terms before publishing or redistributing the library.
