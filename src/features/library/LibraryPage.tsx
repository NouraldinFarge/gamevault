import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ArrowDownAZ, Grid2X2, LayoutList, Search, SlidersHorizontal } from "lucide-react";
import { useMemo, useState } from "react";
import { snapshotKey, useSnapshot } from "../../app/query";
import { getErrorMessage, nativeClient } from "../../shared/lib/native-client";
import { GameCard } from "./GameCard";
import styles from "./LibraryPage.module.css";

const PAGE_SIZE = 24;

export function LibraryPage() {
  const snapshot = useSnapshot();
  const queryClient = useQueryClient();
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const [category, setCategory] = useState("all");
  const [sort, setSort] = useState("title");
  const [view, setView] = useState<"grid" | "list">("grid");
  const [page, setPage] = useState(1);
  const favorite = useMutation({
    mutationFn: nativeClient.toggleFavorite,
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: snapshotKey }),
  });

  const games = snapshot.data?.games ?? [];
  const categories = useMemo(
    () => [...new Set(games.map((game) => game.category).filter(Boolean))].sort(),
    [games],
  );
  const filtered = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return games
      .filter((game) => {
        const matchesQuery =
          !normalizedQuery ||
          game.title.toLowerCase().includes(normalizedQuery) ||
          game.tags.some((tag) => tag.toLowerCase().includes(normalizedQuery));
        const matchesStatus =
          status === "all" ||
          (status === "favorites" && game.favorite) ||
          game.detectionStatus === status;
        return matchesQuery && matchesStatus && (category === "all" || game.category === category);
      })
      .sort((left, right) => {
        if (sort === "recent") {
          return (
            new Date(right.lastPlayedAt ?? right.addedAt).getTime() -
            new Date(left.lastPlayedAt ?? left.addedAt).getTime()
          );
        }
        if (sort === "playtime") return right.playtimeSeconds - left.playtimeSeconds;
        if (sort === "added") {
          return new Date(right.addedAt).getTime() - new Date(left.addedAt).getTime();
        }
        return left.title.localeCompare(right.title);
      });
  }, [category, games, query, sort, status]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount);
  const visible = filtered.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);

  if (snapshot.isLoading) return <div className="page-state">Loading your library...</div>;
  if (snapshot.isError) {
    return (
      <div className="page-state">
        <p className="eyebrow">Library unavailable</p>
        <h1>The local index could not be loaded.</h1>
        <button className="button" type="button" onClick={() => void snapshot.refetch()}>
          Try again
        </button>
      </div>
    );
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <p className="eyebrow">Local collection</p>
          <h1>Library</h1>
          <p>{games.length} indexed games, available offline.</p>
        </div>
        <fieldset className={styles.viewSwitch}>
          <legend className="sr-only">Library layout</legend>
          <button
            type="button"
            className={view === "grid" ? styles.selected : ""}
            aria-label="Grid layout"
            aria-pressed={view === "grid"}
            onClick={() => setView("grid")}
          >
            <Grid2X2 aria-hidden="true" size={17} />
          </button>
          <button
            type="button"
            className={view === "list" ? styles.selected : ""}
            aria-label="List layout"
            aria-pressed={view === "list"}
            onClick={() => setView("list")}
          >
            <LayoutList aria-hidden="true" size={18} />
          </button>
        </fieldset>
      </header>

      <section className={styles.toolbar} aria-label="Library controls">
        <label className={styles.search}>
          <span className="sr-only">Search library</span>
          <Search aria-hidden="true" size={18} />
          <input
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(1);
            }}
            placeholder="Search titles or tags"
          />
        </label>
        <label className={styles.selectControl}>
          <SlidersHorizontal aria-hidden="true" size={16} />
          <span className="sr-only">Filter status</span>
          <select
            value={status}
            onChange={(event) => {
              setStatus(event.target.value);
              setPage(1);
            }}
          >
            <option value="all">All statuses</option>
            <option value="favorites">Favorites</option>
            <option value="detected">Detected</option>
            <option value="configured">Configured</option>
            <option value="missing">Missing</option>
            <option value="unavailable">Unavailable</option>
          </select>
        </label>
        <label className={styles.selectControl}>
          <span className="sr-only">Filter category</span>
          <select
            value={category}
            onChange={(event) => {
              setCategory(event.target.value);
              setPage(1);
            }}
          >
            <option value="all">All categories</option>
            {categories.map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.selectControl}>
          <ArrowDownAZ aria-hidden="true" size={16} />
          <span className="sr-only">Sort library</span>
          <select value={sort} onChange={(event) => setSort(event.target.value)}>
            <option value="title">Title</option>
            <option value="recent">Recently played</option>
            <option value="added">Recently added</option>
            <option value="playtime">Playtime</option>
          </select>
        </label>
      </section>

      {favorite.isError ? (
        <div className="notice error" role="alert">
          {getErrorMessage(favorite.error)}
        </div>
      ) : null}

      {visible.length ? (
        <>
          <div className={styles.results}>
            <span>
              Showing {visible.length} of {filtered.length}
            </span>
            {(query || status !== "all" || category !== "all") && (
              <button
                className="button ghost"
                type="button"
                onClick={() => {
                  setQuery("");
                  setStatus("all");
                  setCategory("all");
                  setPage(1);
                }}
              >
                Clear filters
              </button>
            )}
          </div>
          <div className={styles.games} data-view={view}>
            {visible.map((game) => (
              <GameCard
                key={game.id}
                game={game}
                view={view}
                onFavorite={(id) => favorite.mutate(id)}
                isFavoritePending={favorite.isPending}
              />
            ))}
          </div>
          {pageCount > 1 ? (
            <nav className={styles.pagination} aria-label="Library pages">
              <button
                className="button"
                type="button"
                disabled={safePage <= 1}
                onClick={() => setPage((value) => Math.max(1, value - 1))}
              >
                Previous
              </button>
              <span>
                Page {safePage} of {pageCount}
              </span>
              <button
                className="button"
                type="button"
                disabled={safePage >= pageCount}
                onClick={() => setPage((value) => Math.min(pageCount, value + 1))}
              >
                Next
              </button>
            </nav>
          ) : null}
        </>
      ) : (
        <div className={styles.empty}>
          <Search aria-hidden="true" size={28} />
          <h2>No matching games</h2>
          <p>Change the search or filters, or scan another local folder.</p>
        </div>
      )}
    </div>
  );
}
