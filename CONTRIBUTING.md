# Contributing

GameVault welcomes focused improvements that preserve portable local state, explicit user control, and review before archive extraction, promotion, or execution.

## Before opening a change

1. Search existing [issues](https://github.com/NouraldinFarge/gamevault/issues) and [pull requests](https://github.com/NouraldinFarge/gamevault/pulls).
2. Open a focused issue describing the user problem, expected outcome, and trust-boundary impact.
3. Use synthetic fixtures. Never add commercial games, private archives, credentials, license keys, personal databases, unredacted paths, or generated release builds.
4. Add regression coverage for changes to archive parsing, path handling, promotion, launch, backup restore, portability migration, or metadata allowlists.

Security vulnerabilities belong in [private vulnerability reporting](https://github.com/NouraldinFarge/gamevault/security/advisories/new), not a public issue or pull request.

## Development setup

Required on Windows 10/11 x64: Node.js 24, pnpm 11, the pinned Rust 1.96 MSVC toolchain, Visual Studio C++ Build Tools, Microsoft Edge WebView2, and 7-Zip for archive tests.

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` exposes only synthetic browser-preview data. Use `cargo run --manifest-path src-tauri/Cargo.toml` when testing native filesystem, SQLite, process, or metadata behavior.

## Verification

Run the complete relevant gate before opening a pull request:

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

Run `BUILD-LATEST.ps1` when changing packaging, portable layout, configuration defaults, database location, upgrade behavior, or release automation.

## Trust-boundary checklist

- Archive structure is preflighted before extraction and unsafe packages fail closed.
- Promotion and execution remain separate, explicit user actions.
- Filesystem, process, SQLite, backup, URL, and metadata authority remains in Rust/Tauri.
- Nested archives remain sealed; runtime DLLs remain with their game.
- Metadata is optional and restricted to approved official Steam, GOG, and Epic hosts.
- Logs, screenshots, fixtures, and test output contain no private paths or commercial content.

## Documentation and screenshots

Update public claims whenever behavior changes. Screenshots must use the synthetic preview, a consistent wide-desktop viewport, descriptive alt text, and no personal paths. Keep the home, library, and Local Files images current and remove superseded captures instead of accumulating near-duplicates.

## Pull requests

Keep pull requests small enough to review as one coherent user outcome. Explain tradeoffs, include evidence, call out trust-boundary changes, and let CI, CodeQL, and the locked dependency audit finish before merge.
