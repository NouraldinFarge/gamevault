import { Link } from "@tanstack/react-router";
import { ArrowRight, Clock3, FolderSearch2, Gamepad2, LibraryBig, Star } from "lucide-react";
import { useSnapshot } from "../../app/query";
import { Artwork } from "../../shared/components/Artwork";
import { StatusBadge } from "../../shared/components/StatusBadge";
import { formatDuration, formatRelativeDate } from "../../shared/lib/format";
import type { Game } from "../../shared/lib/types";
import styles from "./HomePage.module.css";

export function HomePage() {
  const snapshot = useSnapshot();
  if (snapshot.isLoading) {
    return <div className="page-state">Loading your local library...</div>;
  }
  if (snapshot.isError || !snapshot.data) {
    return (
      <div className="page-state">
        <p className="eyebrow">Library unavailable</p>
        <h1>GameVault could not read the local index.</h1>
        <button className="button" type="button" onClick={() => void snapshot.refetch()}>
          Try again
        </button>
      </div>
    );
  }

  const { games, stats } = snapshot.data;
  const featured =
    games.find((game) => game.favorite && game.detectionStatus !== "missing") ??
    games.find((game) => game.detectionStatus !== "missing") ??
    null;
  const recent = [...games]
    .filter((game) => game.lastPlayedAt)
    .sort(
      (left, right) =>
        new Date(right.lastPlayedAt ?? 0).getTime() - new Date(left.lastPlayedAt ?? 0).getTime(),
    )
    .slice(0, 4);
  const added = [...games]
    .sort((left, right) => new Date(right.addedAt).getTime() - new Date(left.addedAt).getTime())
    .slice(0, 4);

  return (
    <div className={styles.page}>
      <header className={styles.topline}>
        <div>
          <p className="eyebrow">Your portable collection</p>
          <h1>Good to see you.</h1>
        </div>
        <Link className="button" to="/library">
          <LibraryBig aria-hidden="true" size={17} />
          Browse library
        </Link>
      </header>

      {featured ? (
        <section className={styles.hero} aria-labelledby="featured-title">
          <Artwork
            seed={featured.artworkSeed}
            title={featured.title}
            imageUrl={featured.metadata.heroUrl ?? featured.metadata.coverUrl}
            className={styles.heroArt}
          />
          <div className={styles.heroShade} />
          <div className={styles.heroContent}>
            <p className="eyebrow">Ready to play</p>
            <h2 id="featured-title">{featured.title}</h2>
            <p>{featured.description}</p>
            <div className={styles.heroMeta}>
              <StatusBadge status={featured.detectionStatus} />
              <span>{featured.category}</span>
              <span>{formatDuration(featured.playtimeSeconds)}</span>
            </div>
            <div className={styles.heroActions}>
              <Link
                className="button primary"
                to="/library/$gameId"
                params={{ gameId: featured.id }}
              >
                <Gamepad2 aria-hidden="true" size={18} />
                Open game
              </Link>
              <Link className="button" to="/library/$gameId" params={{ gameId: featured.id }}>
                Details
                <ArrowRight aria-hidden="true" size={16} />
              </Link>
            </div>
          </div>
        </section>
      ) : (
        <section className={styles.emptyHero}>
          <FolderSearch2 aria-hidden="true" size={30} />
          <div>
            <p className="eyebrow">Start your library</p>
            <h2>Scan a local game folder.</h2>
            <p>GameVault indexes metadata only and never changes game files during detection.</p>
          </div>
          <Link className="button primary" to="/files">
            Set up scanning
          </Link>
        </section>
      )}

      <section className={styles.stats} aria-label="Library statistics">
        <StatCard Icon={LibraryBig} value={stats.totalGames} label="Library games" />
        <StatCard Icon={Gamepad2} value={stats.readyGames} label="Ready to launch" />
        <StatCard Icon={Star} value={stats.favorites} label="Favorites" />
        <StatCard
          Icon={Clock3}
          value={formatDuration(stats.totalPlaytimeSeconds)}
          label="Tracked playtime"
        />
      </section>

      <div className={styles.columns}>
        <GameStrip title="Continue playing" games={recent} emptyText="No sessions tracked yet." />
        <GameStrip title="Recently added" games={added} emptyText="No games detected yet." />
      </div>
    </div>
  );
}

function StatCard({
  Icon,
  value,
  label,
}: {
  Icon: typeof LibraryBig;
  value: string | number;
  label: string;
}) {
  return (
    <article className={styles.statCard}>
      <Icon aria-hidden="true" size={18} />
      <strong>{value}</strong>
      <span>{label}</span>
    </article>
  );
}

function GameStrip({
  title,
  games,
  emptyText,
}: {
  title: string;
  games: Game[];
  emptyText: string;
}) {
  return (
    <section className={styles.listSection}>
      <div className="section-heading">
        <h2>{title}</h2>
        <Link to="/library">View all</Link>
      </div>
      {games.length ? (
        <div className={styles.gameList}>
          {games.map((game) => (
            <Link
              key={game.id}
              to="/library/$gameId"
              params={{ gameId: game.id }}
              className={styles.gameRow}
            >
              <Artwork
                compact
                seed={game.artworkSeed}
                title={game.title}
                imageUrl={game.metadata.coverUrl ?? game.metadata.heroUrl}
              />
              <span className={styles.gameRowText}>
                <strong>{game.title}</strong>
                <small>
                  {game.lastPlayedAt
                    ? `Played ${formatRelativeDate(game.lastPlayedAt)}`
                    : `Added ${formatRelativeDate(game.addedAt)}`}
                </small>
              </span>
              <ArrowRight aria-hidden="true" size={16} />
            </Link>
          ))}
        </div>
      ) : (
        <p className={styles.emptyText}>{emptyText}</p>
      )}
    </section>
  );
}
