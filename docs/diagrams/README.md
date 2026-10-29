# Diagrams

These diagrams render directly on GitHub. Each one also has an editable Mermaid source (`.mmd`) that was rendered through draw.io.

## Project map

```mermaid
flowchart TD
    Manifest["Cargo package and workspace"] --> API["Rust API and DLL ABI"]
    API --> DSP["Shared encoder core"]
    DSP --> Packet["32-byte TrueSpeech packet"]
    API --> WAV["RIFF/WAV output"]
    API --> Header["C header and native example"]
    Build["Release builder"] --> Distribution["Windows x64 distribution"]
    Docs["API and architecture guides"] --> Sources["Editable Mermaid sources"]
```

Source: [files-and-apis.mmd](files-and-apis.mmd)

## One-frame encoder algorithm

```mermaid
flowchart TD
    Input["240 mono i16 samples"] --> Scale["Scale and window input"]
    Scale --> Correlation["Autocorrelation and LPC analysis"]
    Correlation --> Quantize["Quantize reflection coefficients"]
    Quantize --> Stability{"LPC stable?"}
    Stability -->|yes| Filter["Build and interpolate LPC filters"]
    Stability -->|no| Safe["Select safe filter coefficients"]
    Safe --> Filter
    Filter --> Weight["Perceptual weighting"]
    Weight --> Pitch["Search open-loop pitch"]
    Pitch --> Subframes["Four 60-sample subframes"]
    Subframes --> Adaptive["Adaptive excitation search"]
    Adaptive --> Pulses["Select seven fixed-codebook pulses"]
    Pulses --> History["Update encoder histories"]
    History --> More{"More subframes?"}
    More -->|yes| Adaptive
    More -->|no| Pack["Pack eight little-endian words"]
    Pack --> Output["32-byte packet"]
```

Source: [encoder-algorithm.mmd](encoder-algorithm.mmd)

## Windows DLL lifecycle

```mermaid
sequenceDiagram
    participant App as Application
    participant ABI as DLL ABI
    participant State as TrueSpeechContext
    App->>ABI: TrueSpeech_Init(format)
    ABI-->>App: State pointer or NULL
    loop Every 240-sample frame
        App->>ABI: TrueSpeech_Encod(state, PCM, packet)
        ABI->>State: Encode one frame
        State-->>ABI: 32-byte packet
        ABI-->>App: Status and packet
    end
    opt Reset stream
        App->>ABI: TrueSpeech_Reset(state)
    end
    App->>ABI: TrueSpeech_Term(state)
```

Source: [dll-and-wav.mmd](dll-and-wav.mmd)

## Function map

```mermaid
mindmap
  root((truespeech functions))
    Rust API
      new / default
      init_from_waveformat
      version / reset
      encoded_packet_bytes
      encode_frame
      encode_samples_into
    Windows DLL
      TrueSpeech_Version
      TrueSpeech_Init
      TrueSpeech_Encod
      TrueSpeech_Reset
      TrueSpeech_Term
    DSP analysis
      normalize_u32 / wrapping_abs_i32
      scale_and_shift_input
      apply_hamming_window / apply_lag_window
      compute_autocorrelation
      levinson_durbin
      quantize_reflection_coeffs
      dequantize_reflection_coeffs
      reflection_to_lpc / check_lpc_stability
    Search and history
      interpolate_lpc_encoder
      perceptual_weighting_filter
      subtract_ringing / impulse_response_energy
      open_loop_pitch_search
      generate_adaptive_vector
      adaptive_codebook_search
      fixed_codebook_mpmlq_search
      update_excitation_history
      update_weighted_synthesis_history
      pack_bitstream
    WAV output
      write_truespeech_wav
      encode_truespeech_wav
      wav_sizes
      TrueSpeechWavError
```

Source: [function-map.mmd](function-map.mmd)

## Read more

- [Architecture and source responsibilities](../architecture.md)
- [Function inputs and outputs](../api.md)
- [Windows C ABI](../windows-abi.md)
- [Documentation index](../README.md)
