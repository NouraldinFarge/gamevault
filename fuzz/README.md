# Archive-path fuzz target

This harness feeds arbitrary UTF-8 data through the 7-Zip listing parser, Windows archive-path validator, and case/separator normalizer. It shares the production source file directly, so the harness cannot drift into a separate implementation.

Run it with a current Rust nightly toolchain and cargo-fuzz 0.13.2:

```powershell
cargo install cargo-fuzz --version 0.13.2 --locked
Set-Location fuzz
cargo +nightly fuzz run archive_paths
```

Only synthetic bytes are accepted. Never add commercial archives, personal paths, credentials, or game content to a corpus or crash artifact.
