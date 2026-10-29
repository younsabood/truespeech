# Windows DLL integration

<p>
  <img alt="Windows x64 DLL" src="https://img.shields.io/badge/Windows-x64-0078D4?style=for-the-badge&amp;logo=windows&amp;logoColor=white">
  <img alt="C interface" src="https://img.shields.io/badge/interface-C%20ABI-2563EB?style=for-the-badge&amp;logo=c&amp;logoColor=white">
</p>

## Build the distribution

On Windows with the MSVC toolchain installed, run:

```powershell
.\build-release.ps1
```

The x64 distribution is written to `dist\windows-x64\`. It contains `truespeech.dll`, the MSVC import library, `truespeech.h`, a C example, this API reference, build details, and SHA-256 checksums.

Compile the C example with MSVC:

```powershell
cl /I dist\windows-x64 examples\use_dll.c dist\windows-x64\truespeech.dll.lib
```

Keep `truespeech.dll` beside the executable or on `PATH` when running it.

## 🔄 Encoder lifecycle

```mermaid
flowchart LR
    App["C or C++ application"] --> Init["TrueSpeech_Init"]
    Init --> State["One state per stream"]
    State --> Encode["TrueSpeech_Encod<br/>240 samples"]
    Encode --> Packet["32-byte packet"]
    State --> Reset["TrueSpeech_Reset<br/>optional"]
    State --> Term["TrueSpeech_Term"]

    classDef app fill:#dbeafe,stroke:#2563eb,color:#10233f,stroke-width:2px
    classDef state fill:#ede9fe,stroke:#7c3aed,color:#24134a,stroke-width:2px
    classDef output fill:#dcfce7,stroke:#16a34a,color:#12351f,stroke-width:2px
    class App,Init app
    class State,Encode,Reset,Term state
    class Packet output
```

```mermaid
sequenceDiagram
    participant App as Application
    participant ABI as TrueSpeech DLL
    participant State as Encoder state
    autonumber
    App->>ABI: TrueSpeech_Init(format descriptors)
    ABI-->>App: Opaque state or NULL
    loop Each 240-sample frame
        App->>ABI: TrueSpeech_Encod(state, PCM, packet)
        ABI->>State: Advance predictor state
        State-->>ABI: 32-byte packet
        ABI-->>App: Status 0 or 1
    end
    opt Start a new stream
        App->>ABI: TrueSpeech_Reset(state)
    end
    App->>ABI: TrueSpeech_Term(state)
```

## Input and output contract

> [!WARNING]
> The C ABI cannot verify pointer validity or buffer length. Every pointer must reference the documented storage, and input, output, and state ranges must not overlap.

- Input is mono signed 16-bit PCM at 8 kHz.
- Each `TrueSpeech_Encod` call reads 240 samples (480 bytes) and writes 32 packet bytes into eight writable `uint32_t` words.
- A context belongs to one stream. Do not share it across concurrent callers.
- Input, output, and state memory ranges must not overlap. All pointers must reference valid storage of the documented size.
- `TrueSpeech_Init` returns a state pointer or `NULL`; `TrueSpeech_Encod` and `TrueSpeech_Reset` return `0` on success and `1` for null arguments; `TrueSpeech_Term` releases the state and returns `0`.
- The C ABI does not decode audio, resample input, or validate every PCM format field.

The exact descriptor layout and function signatures are in [`include/truespeech.h`](../include/truespeech.h) and the [API reference](api.md).
