# Test fixtures

Rust tests create temporary folders and tiny placeholder `.exe` files to test executable scoring, ignored installer names, path handling, SQLite persistence, and metadata updates. No copyrighted game files are used.

Vitest contract and component tests run against the typed local preview client. Playwright drives the synthetic app in Microsoft Edge at desktop and narrow viewports, covers the review-gated archive journey, keyboard navigation, and manual update flow, and runs axe against every primary view. Portable release checks launch `GameVault.exe --health-check` from extracted, renamed, and space-containing folders.
