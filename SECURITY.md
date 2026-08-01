# Security policy

## Supported version

Security fixes are applied to the latest published GameVault release. Upgrade older portable builds before reporting an issue.

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/NouraldinFarge/gamevault/security/advisories/new). Do not file a public issue for path traversal, link or reparse handling, unsafe extraction, command injection, process-launch behavior, allowlist bypass, or credential exposure.

Include the affected version, a minimal reproduction using synthetic content, expected impact, and non-sensitive logs. Do not upload commercial games, private archives, credentials, or personal library databases.

## Trust boundaries

- ZIP archives and extracted files are untrusted until validation and explicit approval complete.
- Archive testing precedes extraction; traversal, link/reparse, and modified-platform markers block promotion.
- Nested archives are not recursively expanded.
- Filesystem, SQLite, metadata, URL, and process operations remain behind the Rust/Tauri authority.
- Optional metadata is restricted to approved official hosts and never becomes a launch dependency.
- GameVault does not download games or establish ownership rights.

