# Quick start

<p>
  <img alt="Rust quick start" src="https://img.shields.io/badge/Rust-quick%20start-E46F00?style=for-the-badge&amp;logo=rust&amp;logoColor=white">
  <img alt="Input format: 8 kHz mono PCM" src="https://img.shields.io/badge/input-8%20kHz%20mono-0891B2?style=for-the-badge">
  <img alt="Output: 32-byte packets" src="https://img.shields.io/badge/output-32%20byte%20packets-16A34A?style=for-the-badge">
</p>

> [!TIP]
> Reuse the same encoder context for every frame in a stream. Reset it when a new stream starts.

## Add the dependency

Add the published crate to your project:

```toml
[dependencies]
truespeech = "1.0.1"
```

## Encode a frame

The encoder expects 240 mono signed 16-bit PCM samples at 8 kHz. Keep the same context for every frame in one stream.

```rust
use truespeech::{TrueSpeechContext, TRUESPEECH_FRAME_SAMPLES, TRUESPEECH_PACKET_BYTES};

let pcm = [0_i16; TRUESPEECH_FRAME_SAMPLES];
let mut encoder = TrueSpeechContext::new();
let mut packet = [0_u8; TRUESPEECH_PACKET_BYTES];

encoder.encode_frame(&pcm, &mut packet);
```

The call returns `()`, writes all 32 packet bytes into the provided array, and advances the predictor state by one frame.

```mermaid
flowchart LR
    PCM["240 mono i16 samples"] --> Context["Persistent TrueSpeechContext"]
    Context --> Packet["Caller-owned 32-byte packet"]
    Packet --> Next["Reuse context for next frame"]

    classDef input fill:#cffafe,stroke:#0891b2,color:#083344,stroke-width:2px
    classDef state fill:#ede9fe,stroke:#7c3aed,color:#24134a,stroke-width:2px
    classDef output fill:#dcfce7,stroke:#16a34a,color:#12351f,stroke-width:2px
    class PCM input
    class Context state
    class Packet,Next output
```

## Encode a longer slice

`encode_samples_into` writes complete packets into caller-owned memory. It pads the final partial frame with zero samples. Each call is a complete operation; it does not retain a partial frame for the next call.

```rust
use truespeech::{EncodeBufferError, TrueSpeechContext};

let samples: Vec<i16> = vec![0; 8_000];
let required = TrueSpeechContext::encoded_packet_bytes(samples.len())
    .ok_or(EncodeBufferError::PacketLengthOverflow)?;
let mut packets = vec![0_u8; required];
let mut encoder = TrueSpeechContext::new();
let bytes_written = encoder.encode_samples_into(&samples, &mut packets)?;
assert_eq!(bytes_written, required);
# Ok::<(), Box<dyn std::error::Error>>(())
```

If the output buffer is too small, the method returns `OutputTooSmall { required, provided }` before changing the encoder or output. It does not append a delayed-tail flush packet.

## Write WAV output

The WAV helpers add RIFF chunks, final-frame padding, and the extra zero frame needed to flush the codec's delayed tail.

```rust
use std::{error::Error, fs::File};
use truespeech::write_truespeech_wav;

fn write_example(samples: &[i16]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create("speech.wav")?;
    write_truespeech_wav(&mut file, samples)?;
    Ok(())
}
```

Use `encode_truespeech_wav(samples)` when you need a complete in-memory `Vec<u8>`. See the [API reference](api.md) for errors and the [architecture guide](architecture.md) for the algorithm stages.
