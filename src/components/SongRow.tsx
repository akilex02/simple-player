import { memo } from "react";
import type { Song } from "../types";
import { formatTime } from "../utils";

interface SongRowProps {
  song: Song;
  isCurrent: boolean;
  coverSrc: string | null;
  onPlay: (song: Song) => void;
}

export const SongRow = memo(({ song, isCurrent, coverSrc, onPlay }: SongRowProps) => (
  <div
    className={`song-row ${isCurrent ? "playing" : ""}`}
    onClick={() => onPlay(song)}
  >
    <div>
      {coverSrc
        ? <img src={coverSrc} alt="Portada" className="cover-thumb" />
        : <div className="cover-thumb" style={{ display: "flex", alignItems: "center", justifyContent: "center", fontSize: "12px" }}>🎵</div>
      }
    </div>
    <div className="song-title-box"><span className="song-title-text">{song.title}</span></div>
    <div className="song-artist-text">{song.artist}</div>
    <div className="song-album-text">{song.album}</div>
    <div className="song-duration-text">{formatTime(song.duration_secs)}</div>
  </div>
));
