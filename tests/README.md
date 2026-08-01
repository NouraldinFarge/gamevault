# Test fixtures

Rust tests create temporary folders and tiny placeholder `.exe` files to test executable scoring, ignored installer names, path handling, SQLite persistence, and metadata updates. No copyrighted game files are used.

Frontend tests run against the typed local preview client. Portable release checks launch `GameVault.exe --health-check` from extracted, renamed, and space-containing folders.

