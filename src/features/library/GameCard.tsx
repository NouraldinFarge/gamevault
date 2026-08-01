import { Link } from "@tanstack/react-router";
import { Clock3, Heart } from "lucide-react";
import { Artwork } from "../../shared/components/Artwork";
import { StatusBadge } from "../../shared/components/StatusBadge";
import { formatDuration } from "../../shared/lib/format";
import type { Game } from "../../shared/lib/types";
import styles from "./GameCard.module.css";

type GameCardProps = {
  game: Game;
  view: "grid" | "list";
  onFavorite(id: string): void;
  isFavoritePending: boolean;
};

export function GameCard({ game, view, onFavorite, isFavoritePending }: GameCardProps) {
  return (
    <article className={styles.card} data-view={view}>
      <Link
        to="/library/$gameId"
        params={{ gameId: game.id }}
        className={styles.coverLink}
        aria-label={`Open ${game.title}`}
      >
        <Artwork
          seed={game.artworkSeed}
          title={game.title}
          imageUrl={game.metadata.coverUrl ?? game.metadata.heroUrl}
          className={styles.artwork}
          compact={view === "list"}
        />
      </Link>
      <div className={styles.body}>
        <div className={styles.topline}>
          <StatusBadge status={game.detectionStatus} />
          <button
            className={styles.favorite}
            type="button"
            aria-label={`${(game.favorite ? "Remove " : "Add ") + game.title} favorite`}
            aria-pressed={game.favorite}
            disabled={isFavoritePending}
            onClick={() => onFavorite(game.id)}
          >
            <Heart aria-hidden="true" size={17} fill={game.favorite ? "currentColor" : "none"} />
          </button>
        </div>
        <Link to="/library/$gameId" params={{ gameId: game.id }} className={styles.title}>
          {game.title}
        </Link>
        <div className={styles.meta}>
          <span>{game.category}</span>
          <span>
            <Clock3 aria-hidden="true" size={13} />
            {formatDuration(game.playtimeSeconds)}
          </span>
        </div>
      </div>
    </article>
  );
}
