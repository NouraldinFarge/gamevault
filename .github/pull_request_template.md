## Purpose

Describe the library, archive-intake, or launch problem addressed.

## Verification

- [ ] `pnpm verify`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml --locked`
- [ ] Relevant hostile-input or boundary tests added
- [ ] No games, archives, credentials, databases, or generated output added

## Safety review

- [ ] Archive validation remains before extraction
- [ ] Promotion and execution still require explicit user review
- [ ] Filesystem and URL inputs remain bounded and allowlisted

