# no_std and WebAssembly

<p>
  <img alt="no_std Rust core" src="https://img.shields.io/badge/Rust-no__std-E46F00?style=for-the-badge&amp;logo=rust&amp;logoColor=white">
  <img alt="WebAssembly target" src="https://img.shields.io/badge/target-WASM-654FF0?style=for-the-badge&amp;logo=webassembly&amp;logoColor=white">
</p>

The `tsd32-encoder-core` workspace member wraps the same DSP source used by the main library. It is `no_std`, does not allocate, and exposes the encoder state and packet functions. The core crate is an `rlib`; it is intended to be linked into a WebAssembly application that supplies its own entry points and memory interface.

> [!IMPORTANT]
> Building this core produces a Rust library artifact, not a standalone `.wasm` module. Your application must provide the entry point, memory, and final `cdylib` link.

Install the target and build the core library:

```powershell
rustup target add wasm32-unknown-unknown
cargo build --locked --release --manifest-path core/Cargo.toml --target wasm32-unknown-unknown
```

This command creates a target-specific Rust library artifact. It does not create a standalone `.wasm` module by itself. Link the core from a `cdylib` or application crate to produce a WebAssembly module.

```mermaid
flowchart LR
    App["WASM application"] --> Core["tsd32-encoder-core"]
    Core --> DSP["Shared encoder.rs DSP"]
    App --> Memory["Caller-managed PCM and packet buffers"]
    Core --> Packet["32-byte TrueSpeech packets"]

    classDef app fill:#dbeafe,stroke:#2563eb,color:#10233f,stroke-width:2px
    classDef core fill:#ede9fe,stroke:#7c3aed,color:#24134a,stroke-width:2px
    classDef data fill:#dcfce7,stroke:#16a34a,color:#12351f,stroke-width:2px
    class App app
    class Core,DSP core
    class Memory,Packet data
```

The core module shares the same `TrueSpeechContext` API and format constants as the main package. The WAV writer and Windows C ABI belong to the standard-library wrapper and are not part of the `no_std` core.

---

Continue to the [API reference](api.md) or return to the [documentation index](README.md).
