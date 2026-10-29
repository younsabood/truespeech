# Contributing

Thanks for helping improve TrueSpeech. Bug reports, documentation fixes, and focused code changes are welcome.

## Before you start

- Search existing issues and pull requests to avoid duplicating work.
- For a substantial change, open an issue first to discuss the intended behavior and API impact.
- Do not include credentials, private recordings, or unrelated generated binaries in a report or pull request.

## Local checks

Install the Rust toolchain and run the checks relevant to your change:

```powershell
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For Windows DLL changes, also build the distribution with `.\build-release.ps1` on a Windows MSVC toolchain. Describe any check you could not run in the pull request.

## Pull requests

- Explain the user-visible change and why it is needed.
- Document public API or behavior changes.
- Add or update documentation when usage, outputs, or error behavior changes.
- Keep each change focused and avoid committing build output or local test recordings.
- Confirm the applicable project license and retain third-party notices when changing sourced material.
