# Quick start

## Add the dependency

Until the crate is published to crates.io, use the public Git repository:

```toml
[dependencies]
truespeech = { git = "https://github.com/younsabood/truespeech" }
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
