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
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo audit --file src-tauri/Cargo.lock
pwsh -NoProfile -File build/test-portable-scripts.ps1 -WorkspaceRoot (Get-Location)
```

Run `BUILD-LATEST.ps1`. It must produce an installer-free x64 portable ZIP, validate a renamed executable from a path containing spaces, preserve portable user state during the active-build transaction, and leave the prior active build recoverable until the new health check passes.

## Publish

- Commit and push the reviewed changes to protected `main`; wait for CI and CodeQL to succeed.
- Create a new signed or annotated tag. Do not move or reuse a released tag.
- The pinned GitHub release workflow must publish the portable ZIP, SHA-256 checksum, SPDX JSON SBOM, source archives, and GitHub build-provenance attestation.
- Use concise notes covering user-visible changes, security boundaries, dependency changes, known limitations, and verification results.

## Verify remotely

- Confirm the release points to the intended commit and is neither a draft nor a prerelease.
- Download the published checksum and ZIP; verify the SHA-256 value and the single versioned top-level folder.
- Verify the GitHub artifact attestation, release assets, CI, CodeQL, and repository default branch.
- Treat the hosted asset checksum and provenance as authoritative. ZIP timestamps are not normalized yet, so separate local and hosted builds are not claimed to be byte-for-byte reproducible.
- Confirm the README version/download link, About description, homepage, topics, and portfolio links are current.
- Confirm the local worktree is clean and `main` exactly matches `origin/main`.
