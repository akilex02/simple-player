import { memo, startTransition } from "react";
import type { Song } from "../types";
import { getCoverUrl } from "../utils";

interface ArtistGroup {
  artist: string;
  count: number;
  representativeSong: Song | null;
}

interface ArtistsGridProps {
  artistGroups: ArtistGroup[];
  artistSortOrder: "asc" | "desc";
  onToggleSortOrder: () => void;
  onSelectArtist: (artist: string) => void;
}

export const ArtistsGrid = memo(function ArtistsGrid({
  artistGroups, artistSortOrder, onToggleSortOrder, onSelectArtist,
}: ArtistsGridProps) {
  return (
    <>
      <div className="artists-header-bar">
        <span className="artists-header-title">Artistas ({artistGroups.length})</span>
        <button
          className="btn-sort-artist"
          onClick={onToggleSortOrder}
          title="Cambiar orden alfabético de los artistas"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M3 18h6v-2H3v2zM3 6v2h18V6H3zm0 7h12v-2H3v2z" />
          </svg>
          <span>{artistSortOrder === "asc" ? "Orden: A ➔ Z (Ascendente)" : "Orden: Z ➔ A (Descendente)"}</span>
        </button>
      </div>
      <div className="artists-grid">
        {artistGroups.map(item => {
          const coverSrc = getCoverUrl(item.representativeSong?.cover_art ?? null);
          return (
            <div
              key={item.artist}
              className="artist-card"
              onClick={() => startTransition(() => onSelectArtist(item.artist))}
            >
              {coverSrc
                ? <img src={coverSrc} alt={item.artist} className="artist-avatar" />
                : <div className="artist-avatar">🎤</div>
              }
              <span className="artist-name">{item.artist}</span>
              <span className="artist-song-count">{item.count} {item.count === 1 ? "canción" : "canciones"}</span>
            </div>
          );
        })}
      </div>
    </>
  );
});
