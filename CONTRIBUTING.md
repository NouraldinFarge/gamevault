# Contributing

GameVault welcomes focused bug reports and improvements that preserve its review-before-execution model.

## Before opening a change

1. Open an issue describing the user problem and any trust-boundary impact.
2. Use synthetic fixtures. Do not add commercial games, archives, credentials, license keys, personal databases, or generated builds.
3. Add regression tests for changes to archive parsing, path handling, promotion, metadata allowlists, or process launch.

## Local verification

```powershell
pnpm install --frozen-lockfile
pnpm verify
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
pnpm audit
pnpm audit --prod
cargo audit --file src-tauri/Cargo.lock
pwsh -NoProfile -File build/test-portable-scripts.ps1 -WorkspaceRoot (Get-Location)
```
