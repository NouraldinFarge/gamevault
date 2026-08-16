import fc from "fast-check";
import { describe, expect, it } from "vitest";
import { emptyGameMetadata, type Game } from "../../shared/lib/types";
import { filterAndSortGames, paginateGames } from "./library-query";

function game(index: number): Game {
  return {
    id: `game-${index}`,
    title: `Game ${index.toString().padStart(5, "0")}`,
    description: "Synthetic scale fixture",
    installPath: `C:\\GameVault\\Games\\Game ${index}`,
    executablePath: `C:\\GameVault\\Games\\Game ${index}\\Game.exe`,
    launchArgs: [],
    tags: index % 2 === 0 ? ["Even"] : ["Odd"],
    category: index % 3 === 0 ? "Strategy" : "Action",
    favorite: index % 10 === 0,
    detectionStatus: index % 7 === 0 ? "missing" : "detected",
    detectionSource: "synthetic-test",
    folderSizeBytes: index * 1024,
    lastPlayedAt: null,
    playtimeSeconds: index * 60,
    addedAt: new Date(Date.UTC(2026, 0, 1) + index * 1000).toISOString(),
    updatedAt: "2026-01-01T00:00:00Z",
    contentSignature: `fixture-${index}`,
    artworkSeed: index,
    metadata: structuredClone(emptyGameMetadata),
  };
}

describe("library queries", () => {
  it("keeps a 10,000-game collection bounded to one rendered page", () => {
    const games = Array.from({ length: 10_000 }, (_, index) => game(index));
    const started = performance.now();
    const filtered = filterAndSortGames(games, {
      query: "",
      status: "all",
      category: "all",
      sort: "title",
    });
    const result = paginateGames(filtered, 1, 24);
    const elapsed = performance.now() - started;

    expect(result.total).toBe(10_000);
    expect(result.items).toHaveLength(24);
    expect(result.pageCount).toBe(417);
    expect(games[0].id).toBe("game-0");
    expect(elapsed).toBeLessThan(5_000);
  });

  it("filters title, tags, favorite status, and category together", () => {
    const result = filterAndSortGames([game(9), game(10), game(12)], {
      query: "even",
      status: "favorites",
      category: "Action",
      sort: "playtime",
    });

    expect(result.map((item) => item.id)).toEqual(["game-10"]);
  });

  it("never exposes more than the requested bounded page", () => {
    fc.assert(
      fc.property(
        fc.array(fc.integer(), { maxLength: 2_000 }),
        fc.integer({ min: -1_000, max: 1_000 }),
        fc.integer({ min: 1, max: 100 }),
        (items, page, pageSize) => {
          const result = paginateGames(items, page, pageSize);
          expect(result.items.length).toBeLessThanOrEqual(pageSize);
          expect(result.total).toBe(items.length);
          expect(result.page).toBeGreaterThanOrEqual(1);
          expect(result.page).toBeLessThanOrEqual(result.pageCount);
        },
      ),
    );
  });
});
