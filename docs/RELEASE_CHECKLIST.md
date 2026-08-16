# Release checklist

Use this checklist from a clean `main` checkout. Existing immutable tags and releases are never replaced.

## Prepare

- Confirm `main` is synchronized with `origin/main` and the worktree contains only the intended release changes.
- Review open pull requests, Dependabot proposals, CodeQL, branch protection, workflow permissions, and pinned action commits.
- Set the same semantic version in `VERSION`, `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, the browser-preview health fixture, README, and changelog.
- Confirm the tag will be exactly `v` plus the value in `VERSION`.

## Verify

```powershell
pnpm install --frozen-lockfile
pnpm audit
pnpm audit --prod
pnpm verify
pnpm exec playwright install chromium
pnpm test:e2e
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
$env:RUSTFLAGS = "-Dwarnings"
cargo check --manifest-path fuzz/Cargo.toml --locked --all-targets
cargo audit --file src-tauri/Cargo.lock
pwsh -NoProfile -File build/test-portable-scripts.ps1 -WorkspaceRoot (Get-Location)
```

Run `BUILD-LATEST.ps1`. It must produce an installer-free x64 portable ZIP, validate a renamed executable from a path containing spaces, preserve portable user state during the active-build transaction, and leave the prior active build recoverable until the new health check passes.

For an unsigned release, confirm the three `GAMEVAULT_SIGNING_*` values are absent. For a signed release, configure the PFX and password as protected secrets and the HTTPS RFC 3161 timestamp URL as a reviewed repository variable. The build must report a successful `signtool verify`; never print or copy certificate material into logs or repository files.

## Publish

- Commit and push the reviewed changes to protected `main`; wait for CI and CodeQL to succeed.
- Create a new signed or annotated tag. Do not move or reuse a released tag.
- The pinned GitHub release workflow must publish the portable ZIP, SHA-256 checksum, SPDX JSON SBOM, source archives, and GitHub build-provenance attestation.
- Use concise notes covering user-visible changes, security boundaries, dependency changes, known limitations, and verification results.

## Verify remotely

- Confirm the release points to the intended commit and is neither a draft nor a prerelease.
- Download the published checksum and ZIP; verify the SHA-256 value and the single versioned top-level folder.
- Verify the GitHub artifact attestation, release assets, CI, CodeQL, and repository default branch.
- Treat the hosted asset checksum and provenance as authoritative. ZIP ordering, timestamps, and entry metadata are normalized, and identical staged inputs are tested to produce identical ZIP bytes. Independent source builds are not yet claimed to be byte-for-byte identical because compiler-produced artifacts may vary.
- Confirm the README version/download link, About description, homepage, topics, and portfolio links are current.
- Confirm the local worktree is clean and `main` exactly matches `origin/main`.
