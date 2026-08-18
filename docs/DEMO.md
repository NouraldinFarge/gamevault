# Synthetic demonstration

The public GameVault demo runs the React interface with fictional games, procedural artwork, and in-memory responses. It has no Tauri authority and cannot read local folders, inspect archives, launch programs, download files, or modify the computer.

For a shorter overview, watch the [silent 45-second walkthrough](media/gamevault-product-tour.mp4) or read its [time-coded transcript](media/gamevault-product-tour-transcript.md).

## Five-minute tour

1. **Home** — review the featured title, portable-library statistics, recent additions, and continuation shortcuts.
2. **Library** — search, filter, sort, change layout, favorite a title, and open its details.
3. **Game details** — edit personal metadata and preview the explicit official-store matching workflow.
4. **Local files** — inspect the managed folder model, reopen a synthetic Staging package, preview exact file changes, review prerequisite evidence, and inspect persistent operation history.
5. **Settings** — review portable paths, diagnostics, database backup controls, the manual-only update check, and product boundaries.

The demo deliberately cannot prove native archive handling. The Rust tests, release checks, and downloadable Windows build provide that evidence; see the [README evidence table](../README.md#evidence-not-promises) and [architecture](ARCHITECTURE.md).

## Privacy and content

- No commercial games, covers, archives, credentials, license data, or personal library records are included.
- All names, durations, paths, metadata, and operation results are synthetic.
- Official-provider buttons open only public Steam, GOG, or Epic search pages.
- Refreshing the page resets the in-memory demonstration state.
