## User outcome

<!-- Explain the user-visible problem and the result of this change. -->

## Scope

<!-- List the important implementation, documentation, or workflow changes. -->

## Evidence

<!-- Include focused test output, synthetic fixtures, or redacted before/after screenshots. -->

- [ ] `pnpm verify`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check`
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] Full dependency audits pass when a lockfile changed
- [ ] Portable build/state-preservation probes pass when release behavior changed

## Trust-boundary review

- [ ] ZIP validation still precedes extraction
- [ ] Promotion and execution still require explicit user review
- [ ] Filesystem, process, SQLite, backup, URL, and metadata inputs remain bounded behind Rust/Tauri authority
- [ ] Metadata remains optional and restricted to approved official hosts
- [ ] No commercial games, private archives, credentials, databases, personal paths, or generated builds were added

## Presentation and documentation

- [ ] User-facing behavior and limitations are documented
- [ ] Screenshots use synthetic data, accurate labels, descriptive alt text, and no personal paths
- [ ] Release notes or the changelog are updated when appropriate
