# Release status

## Public artifact

The latest downloadable build is the immutable [`v0.3.5` Windows release](https://github.com/NouraldinFarge/gamevault/releases/tag/v0.3.5). The portable ZIP, checksum, SBOM, and provenance attached to that release describe the `v0.3.5` tag—not later source changes.

The machine-readable [`release-status.json`](../release-status.json) keeps public, source, and target identities separate. `VERSION`, `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and the browser health fixture are aligned at `0.4.0-dev.0`, so any local package built from current source carries an explicit development identity instead of masquerading as the immutable public release. `targetVersion` is `0.4.0`, `latestPublicVersion` remains `0.3.5`, and `sourceStatus` remains `unreleased`.

## Development milestone

The first changelog section records `0.4.0 - Unreleased`. It groups the post-`v0.3.5` work now visible in source and in the zero-authority browser preview. A local build identifies itself as `0.4.0-dev.0`; that does **not** imply that a stable `v0.4.0` tag, public Windows archive, checksum, SBOM, attestation, or GitHub Release exists.

## Promotion boundary

Before `0.4.0` can become public, a clean release commit must:

1. change every source version from `0.4.0-dev.0` to stable `0.4.0`, date the changelog entry, and set `sourceStatus` to `release-ready`;
2. pass the complete JavaScript, browser, Rust, fuzz, dependency, portable-build, and privacy/media gates in [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md);
3. pass required CI, CodeQL, and dependency checks on the exact protected commit;
4. publish a new immutable `v0.4.0` tag and release without altering `v0.3.5`;
5. verify the public download, checksum, attestation, README, demo, and release links from outside the owner session.

The release workflow rejects prerelease versions and any source state other than `release-ready`. Until all five conditions are recorded, repository documentation must continue to call `0.4.0-dev.0` a source-only development build and send Windows users to `v0.3.5`.
