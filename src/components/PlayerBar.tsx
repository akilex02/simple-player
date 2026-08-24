import type { RepeatMode, Song } from "../types";
import { ProgressBar } from "./ProgressBar";
import { TransportButtons } from "./TransportButtons";
import { VolumeControl } from "./VolumeControl";

interface PlayerBarProps {
  currentSong: Song | null;
  currentCoverSrc: string | null;
  currentTime: number;
  isPlaying: boolean;
  isShuffle: boolean;
  repeatMode: RepeatMode;
  isNormalizeVolume: boolean;
  isMuted: boolean;
  volume: number;
  showLyrics: boolean;
  onToggleShuffle: () => void;
  onPrev: () => void;
  onTogglePlay: () => void;
  onNext: () => void;
  onToggleRepeat: () => void;
  onSeekDragStart: () => void;
  onSeekScrub: (secs: number) => void;
  onSeekCommit: (secs: number) => void;
  onToggleNormalize: () => void;
  onToggleMute: () => void;
  onVolumeChange: (vol: number) => void;
  onVolumeWheel: (e: React.WheelEvent) => void;
  onOpenLyrics: () => void;
  onOpenFullscreen: () => void;
}

export function PlayerBar({
  currentSong, currentCoverSrc, currentTime, isPlaying, isShuffle, repeatMode,
  isNormalizeVolume, isMuted, volume, showLyrics,
  onToggleShuffle, onPrev, onTogglePlay, onNext, onToggleRepeat,
  onSeekDragStart, onSeekScrub, onSeekCommit,
  onToggleNormalize, onToggleMute, onVolumeChange, onVolumeWheel,
  onOpenLyrics, onOpenFullscreen,
}: PlayerBarProps) {
  return (
    <div className="player-bar">
      <div className="current-track-info">
        {currentCoverSrc
          ? <img src={currentCoverSrc} alt="Portada" className="current-cover" />
          : <div className="current-cover" style={{ display: "flex", alignItems: "center", justifyContent: "center", fontSize: "24px" }}>🎧</div>
        }
        <div className="current-details">
          <span className="current-title">{currentSong ? currentSong.title : "Ninguna canción"}</span>
          <span className="current-artist">{currentSong ? currentSong.artist : "Selecciona un tema para iniciar"}</span>
        </div>
      </div>

      <div className="player-controls-center">
        <TransportButtons
          size="compact"
          isShuffle={isShuffle}
          isPlaying={isPlaying}
          repeatMode={repeatMode}
          onToggleShuffle={onToggleShuffle}
          onPrev={onPrev}
          onTogglePlay={onTogglePlay}
          onNext={onNext}
          onToggleRepeat={onToggleRepeat}
        />

        <ProgressBar
          currentTime={currentTime}
          duration={currentSong?.duration_secs ?? 0}
          onDragStart={onSeekDragStart}
          onScrub={onSeekScrub}
          onCommit={onSeekCommit}
        />
      </div>

      <div className="player-extra-controls">
        <VolumeControl
          size="compact"
          isNormalizeVolume={isNormalizeVolume}
          onToggleNormalize={onToggleNormalize}
          isMuted={isMuted}
          volume={volume}
          onToggleMute={onToggleMute}
          onVolumeChange={onVolumeChange}
          onWheel={onVolumeWheel}
        />

        <button className={`btn-icon ${showLyrics ? "active" : ""}`} onClick={onOpenLyrics} title="Ver letra">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M14 17H4v2h10v-2zm6-8H4v2h16V9zM4 15h16v-2H4v2zM4 5v2h16V5H4z" /></svg>
        </button>

        <button className="btn-icon" onClick={onOpenFullscreen} title="Pantalla Completa">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z" /></svg>
        </button>
      </div>
    </div>
  );
}
