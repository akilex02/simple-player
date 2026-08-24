import { memo, startTransition } from "react";
import { useVirtualRows } from "../hooks/useVirtualRows";
import type { Song, SortField } from "../types";
import { getCoverUrl } from "../utils";
import { SongRow } from "./SongRow";

// cover-thumb (44px) + padding vertical (12px x2) + margin-bottom (6px) de .song-row
const ROW_HEIGHT = 74;

interface SongTableProps {
  songs: Song[];
  currentSong: Song | null;
  selectedArtist: string | null;
  sortField: SortField;
  sortDirection: "asc" | "desc";
  loading: boolean;
  currentFolderPath: string | null;
  onBackToArtists: () => void;
  onSort: (field: SortField) => void;
  onPlaySong: (song: Song, sourceQueue: Song[]) => void;
  onShufflePlay: (songs: Song[]) => void;
  onSelectFolder: () => void;
}

export const SongTable = memo(function SongTable({
  songs, currentSong, selectedArtist, sortField, sortDirection, loading,
  currentFolderPath, onBackToArtists, onSort, onPlaySong, onShufflePlay, onSelectFolder,
}: SongTableProps) {
  const renderSortIndicator = (field: SortField) => {
    if (sortField !== field) return null;
    return sortDirection === "asc" ? " ▲" : " ▼";
  };

  const { containerRef, range } = useVirtualRows(songs.length, ROW_HEIGHT, ".song-list-container");
  const visibleSongs = songs.slice(range.start, range.end);

  return (
    <>
      {selectedArtist && (
        <div className="artist-view-header">
          <button className="btn-back" onClick={() => startTransition(onBackToArtists)}>
            ← Volver a Artistas
          </button>
          <h2>{selectedArtist}</h2>
        </div>
      )}

      {songs.length > 0 && (
        <button className="btn-shuffle-hero" onClick={() => onShufflePlay(songs)}>
          <svg width="18" height="18" viewBox="0 0 24 24" fill="white"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.45 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" /></svg>
          <span>{selectedArtist ? `Reproducción Aleatoria de ${selectedArtist}` : "Reproducción Aleatoria de toda la música"}</span>
        </button>
      )}

      {songs.length > 0 ? (
        <>
          <div className="song-list-header">
            <span>#</span>
            <span className="sort-header-btn" onClick={() => onSort("title")}>Título{renderSortIndicator("title")}</span>
            <span className="sort-header-btn" onClick={() => onSort("artist")}>Artista{renderSortIndicator("artist")}</span>
            <span className="sort-header-btn" onClick={() => onSort("album")}>Álbum{renderSortIndicator("album")}</span>
            <span className="sort-header-btn" onClick={() => onSort("duration")}>Duración{renderSortIndicator("duration")}</span>
          </div>
          <div ref={containerRef} style={{ position: "relative", height: songs.length * ROW_HEIGHT }}>
            <div style={{ position: "absolute", top: range.start * ROW_HEIGHT, left: 0, right: 0 }}>
              {visibleSongs.map(song => (
                <SongRow
                  key={song.path}
                  song={song}
                  isCurrent={currentSong !== null && currentSong.path === song.path}
                  coverSrc={getCoverUrl(song.cover_art)}
                  onPlay={s => onPlaySong(s, songs)}
                />
              ))}
            </div>
          </div>
        </>
      ) : (
        <div className="empty-state">
          <div className="empty-state-icon">🎵</div>
          <p>{currentFolderPath ? `No hay canciones en "${currentFolderPath}"` : "No se encontraron canciones"}</p>
          <button className="btn-scan" onClick={onSelectFolder} disabled={loading}>Seleccionar carpeta de música</button>
        </div>
      )}
    </>
  );
});
