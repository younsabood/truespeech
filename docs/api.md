# API and output reference

## Audio contract

The encoder accepts mono signed 16-bit PCM at 8,000 Hz. One input frame contains 240 samples (30 ms) and produces one 32-byte TrueSpeech packet. The encoder is stateful: keep one context per stream and feed its frames in order. Reset or create a new context when starting a separate stream.

The Rust and C APIs encode packets only. They do not resample, decode, or add a WAV delay-flush packet. The WAV helpers handle padding, RIFF chunks, and the codec's delayed tail.

## Rust API

The following items are re-exported from the crate root. DSP constants and `pack_bitstream` are also available through the `encoder` module.

| Item | Input | Output and state changes |
| --- | --- | --- |
| `TRUESPEECH_SAMPLE_RATE` | — | `u32`, 8,000 samples per second. |
| `TRUESPEECH_FRAME_SAMPLES` | — | `usize`, 240 samples per encoder frame. |
| `TRUESPEECH_SUBFRAME_SAMPLES` | — | `usize`, 60 samples per subframe. |
| `TRUESPEECH_PACKET_BYTES` | — | `usize`, 32 bytes per encoded frame. |
| `TRUESPEECH_VERSION` | — | `u32`, version value `0x01000000`. |
| `TrueSpeechContext::new()` / `Default::default()` | — | A fresh encoder state with zeroed history and initialized codec tables. |
| `TrueSpeechContext::init_from_waveformat(a, b)` | Two `&[i16]` descriptors | `Some(context)` for one PCM tag (`0x0001`) and one TrueSpeech tag (`0x0022`), with TrueSpeech descriptor entries 9 and 10 equal to 1 and 240. Returns `None` for invalid or short descriptors. |
| `TrueSpeechContext::version()` | — | The codec version as `u32`. |
| `context.reset()` | Mutable context | Returns `()`. Clears predictor, excitation, and filter history. |
| `TrueSpeechContext::encoded_packet_bytes(count)` | PCM sample count | `Some(bytes)` for the number of packets needed, rounding a partial final frame up; `None` if the byte count overflows `usize`. |
| `context.encode_frame(pcm, packet)` | `&[i16; 240]`, `&mut [u8; 32]` | Returns `()`, fills one packet, and advances the encoder state by one frame. |
| `context.encode_samples_into(samples, output)` | PCM slice and caller-owned byte buffer | `Ok(bytes_written)` on success. Zero-pads the final partial frame. Returns `PacketLengthOverflow` or `OutputTooSmall { required, provided }`; a capacity error occurs before changing the state or output. This operation does not append a delay-flush frame. |
| `pack_bitstream(params, output)` | `&FrameParams`, `&mut [u8; 32]` | Returns `()`, writes the parameter fields into one packet in codec byte order. |
| `write_truespeech_wav(writer, samples)` | Any `Write`, PCM slice | `Ok(())` after writing a complete RIFF/WAV stream. Returns `TrueSpeechWavError` for a RIFF size overflow or writer error. It does not collect all packets in memory. |
| `encode_truespeech_wav(samples)` | PCM slice | `Ok(Vec<u8>)` containing the complete RIFF/WAV file, or `TooLarge`, `AllocationFailed`, or `Io` error. |

### Error values

`EncodeBufferError` has these variants:

- `PacketLengthOverflow`: the requested packet byte count cannot fit in `usize`.
- `OutputTooSmall { required, provided }`: the provided packet buffer is short. Both fields are byte counts. The encoder state and output remain unchanged.

`TrueSpeechWavError` has these variants:

- `TooLarge`: a WAV size or sample count exceeds the RIFF 32-bit size limit.
- `AllocationFailed`: memory reservation failed in the in-memory WAV helper.
- `Io(error)`: the destination writer returned an I/O error; `source()` exposes it.

## Packet and WAV outputs

Each packet is exactly 32 bytes (256 bits), represented as eight 32-bit words packed in little-endian byte order. The Rust packet APIs write bytes into a caller-owned array. The Windows C ABI accepts a pointer to eight `uint32_t` words; on supported Windows targets, their in-memory byte representation is the packet byte stream.

The WAV helpers write a RIFF file with the TrueSpeech format tag `0x0022`, an extended 32-byte `fmt` chunk, a `fact` chunk, and a `data` chunk of complete 32-byte packets. A final partial input frame is padded with zero samples. The `fact` sample count includes the encoder's 60-sample delay. The writer appends a zero frame when needed to flush delayed audio while keeping the declared playback duration aligned with the original input plus that delay.

## Windows C ABI

The declarations and constants are in `include/truespeech.h`. Each function uses the system ABI. Pass one encoder state per stream, and release it exactly once.

| Function | Input | Output |
| --- | --- | --- |
| `TrueSpeech_Version()` | None | Codec version `0x01000000`. |
| `TrueSpeech_Init(input_format, output_format)` | Two pointers to format descriptors. The descriptors may be supplied in either PCM/TrueSpeech order. The PCM descriptor must expose its first `int16_t`; the TrueSpeech descriptor must expose at least eleven `int16_t` values. | Opaque state pointer, or `NULL` if a pointer, format tag, revision, or sample-block value is invalid. Entries 9 and 10 of the TrueSpeech descriptor must be `1` and `240`. The rest of the PCM descriptor is not validated. |
| `TrueSpeech_Encod(state, pcm, packet)` | Live state, 240 readable `int16_t` samples (480 bytes), and storage for eight writable `uint32_t` values. The PCM pointer may be unaligned. | Writes one packet. Returns `TRUESPEECH_STATUS_OK` (`0`) on success or `TRUESPEECH_STATUS_INVALID_ARGUMENT` (`1`) when an argument is null. |
| `TrueSpeech_Reset(state)` | Live state pointer | Clears encoder history and returns `0`; null returns `1`. |
| `TrueSpeech_Term(state)` | State created by `TrueSpeech_Init`, or null | Releases a non-null state and returns `0`. The pointer must not be reused after this call. |

The state, PCM input, and packet output memory ranges passed to `TrueSpeech_Encod` must not overlap. Every non-null pointer must address the documented readable or writable storage. The C ABI cannot validate arbitrary pointer validity or buffer length.

## Encoder algorithm stages

The frame path processes four 60-sample subframes. Internal helper names are implementation details; their outputs describe the intermediate data passed to the following stage.

| Stage and helpers | Result passed forward |
| --- | --- |
| `scale_and_shift_input`, `apply_hamming_window` | Scaled analysis samples and a windowed 240-sample frame; overlap state is advanced. |
| `compute_autocorrelation`, `apply_lag_window`, `levinson_durbin` | Nine autocorrelation terms, windowed lag values, and eight LPC reflection coefficients. |
| `quantize_reflection_coeffs`, `dequantize_reflection_coeffs`, `reflection_to_lpc`, `check_lpc_stability` | Quantized LPC indices and a stable 8-coefficient synthesis filter, with fallback values when analysis is unstable. |
| `interpolate_lpc_encoder`, `perceptual_weighting_filter`, `subtract_ringing`, `impulse_response_energy` | Subframe LPC filters and weighted target/energy values used by pitch and pulse selection. |
| `open_loop_pitch_search`, `generate_adaptive_vector`, `adaptive_codebook_search` | Pitch lags, adaptive excitation vectors, and adaptive-codebook indices for each subframe. |
| `fixed_codebook_mpmlq_search` | Seven-pulse positions, pulse signs, and gain indices for each subframe. |
| `update_excitation_history`, `update_weighted_synthesis_history` | Updated state used by the next subframe and frame. |
| `pack_bitstream` | The eight packed words written as a 32-byte packet. |

The arithmetic helpers `normalize_u32` and `wrapping_abs_i32` return normalized signed values used by the DSP search routines. `FrameParams` contains the quantized LPC, pitch, adaptive-gain, pulse-position, pulse-sign, and interpolation fields for the current frame.

## Related documentation

- [Quick start](quickstart.md)
- [Architecture](architecture.md)
- [Windows integration](windows-abi.md)
- [no_std and WebAssembly](wasm.md)
- [Rendered diagrams](diagrams/README.md)
