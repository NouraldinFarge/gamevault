import type { Game } from "../../shared/lib/types";

export type LibrarySort = "title" | "recent" | "added" | "playtime";

export type LibraryFilters = {
  query: string;
  status: string;
  category: string;
  sort: LibrarySort;
};

export function collectCategories(games: readonly Game[]) {
  return [...new Set(games.map((game) => game.category).filter(Boolean))].sort();
}

export function filterAndSortGames(games: readonly Game[], filters: LibraryFilters) {
  const normalizedQuery = filters.query.trim().toLowerCase();
  return games
    .filter((game) => {
      const matchesQuery =
        !normalizedQuery ||
        game.title.toLowerCase().includes(normalizedQuery) ||
        game.tags.some((tag) => tag.toLowerCase().includes(normalizedQuery));
      const matchesStatus =
        filters.status === "all" ||
        (filters.status === "favorites" && game.favorite) ||
        game.detectionStatus === filters.status;
      const matchesCategory = filters.category === "all" || game.category === filters.category;
      return matchesQuery && matchesStatus && matchesCategory;
    })
    .sort((left, right) => {
      if (filters.sort === "recent") {
        return (
          dateValue(right.lastPlayedAt ?? right.addedAt) -
          dateValue(left.lastPlayedAt ?? left.addedAt)
        );
      }
      if (filters.sort === "playtime") return right.playtimeSeconds - left.playtimeSeconds;
      if (filters.sort === "added") return dateValue(right.addedAt) - dateValue(left.addedAt);
      return left.title.localeCompare(right.title);
    });
}

export function paginateGames<T>(items: readonly T[], requestedPage: number, pageSize: number) {
  if (!Number.isInteger(pageSize) || pageSize < 1 || pageSize > 100) {
    throw new Error("Page size must be an integer from 1 through 100.");
  }
  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const page = Math.min(Math.max(1, Math.trunc(requestedPage) || 1), pageCount);
  const start = (page - 1) * pageSize;
  return {
    items: items.slice(start, start + pageSize),
    page,
    pageCount,
    total: items.length,
  };
}

function dateValue(value: string) {
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : 0;
}
