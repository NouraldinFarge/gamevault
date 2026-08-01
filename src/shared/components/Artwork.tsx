import { type CSSProperties, useState } from "react";
import styles from "./Artwork.module.css";

type ArtworkProps = {
  seed: number;
  title: string;
  className?: string;
  compact?: boolean;
  imageUrl?: string | null;
};

export function Artwork({
  seed,
  title,
  className = "",
  compact = false,
  imageUrl = null,
}: ArtworkProps) {
  const palette = Math.abs(seed) % 8;
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  const showImage = !!imageUrl && imageUrl !== failedUrl;
  return (
    <div
      className={[styles.artwork, compact ? styles.compact : "", className].join(" ")}
      data-palette={palette}
      role="img"
      aria-label={showImage ? `Official store artwork for ${title}` : `Abstract cover for ${title}`}
      style={{ "--art-angle": `${112 + (seed % 37)}deg` } as CSSProperties}
    >
      {showImage ? (
        <img
          className={styles.storeImage}
          src={imageUrl}
          alt=""
          loading={compact ? "lazy" : "eager"}
          referrerPolicy="no-referrer"
          onError={() => setFailedUrl(imageUrl)}
        />
      ) : null}
      <span className={styles.orb} />
      <span className={styles.grid} />
      <span className={styles.monogram} aria-hidden="true" data-hidden={showImage}>
        {title
          .split(/\s+/)
          .slice(0, 2)
          .map((word) => word[0])
          .join("")
          .toUpperCase()}
      </span>
    </div>
  );
}
