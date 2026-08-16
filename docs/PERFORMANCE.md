# Performance and scale evidence

GameVault keeps the interactive library bounded even when the local catalog grows. Search, filter, and sort run over in-memory metadata; React receives at most 24 game cards for the current page.

## Reproducible scale fixture

`src/features/library/library-query.test.ts` builds 10,000 synthetic games, applies the same production query function used by the Library page, and verifies:

- all 10,000 records remain represented in the result count;
- only 24 records enter the rendered page;
- the page count is correct;
- the source collection is not mutated; and
- the operation completes within a deliberately generous five-second regression ceiling on CI.

The same test file uses generated arrays, page requests, and page sizes to prove pagination never returns more than its configured maximum and always clamps the current page to a valid range.

## Deliberate bounds

| Boundary | Current limit | Reason |
| --- | ---: | --- |
| Rendered library page | 24 games | Keeps DOM, artwork, and focus order predictable |
| User-selected page size in query helper | 100 maximum | Prevents accidental unbounded use by future callers |
| Archive structural entries | 100,000 | Bounds listing, validation, and extraction review |
| File-diff manifest | 50,000 entries per tree | Bounds hashing and review data for install/update approval |
| Operation history shown | 12 recent records | Keeps recovery evidence useful without overwhelming the view |

This is a regression guard, not a universal hardware benchmark. Disk scanning, hashing, archive testing, and extraction remain dominated by storage speed and archive size; the interface reports those operations separately and never treats the 10,000-record metadata test as evidence for filesystem throughput.
