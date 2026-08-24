import { useEffect, useState } from "react";
import { useSpectrumData } from "../hooks/useSpectrumData";
import type { Lyrics } from "../hooks/useLyrics";
import type { RepeatMode, Song } from "../types";
import { LyricsPanel } from "./LyricsPanel";
import { ProgressBar } from "./ProgressBar";
import { TransportButtons } from "./TransportButtons";
import { VISUALIZER_LABELS, VISUALIZER_MODES, VisualizerBackground, type VisualizerMode } from "./VisualizerBackground";
import { VolumeControl } from "./VolumeControl";

// Debe coincidir con la duración del transition en .fullscreen-lyrics-column (App.css)
const LYRICS_TRANSITION_MS = 300;

/** Mantiene la columna montada durante el fade-out y agrega "visible" un frame después
 *  del mount para que el navegador anime desde el estado oculto (opacity/translateX). */
function useLyricsColumnTransition(showLyrics: boolean) {
  const [mounted, setMounted] = useState(showLyrics);
  const [entered, setEntered] = useState(false);

  useEffect(() => {
    if (showLyrics) {
      setMounted(true);
      const raf = requestAnimationFrame(() => setEntered(true));
      return () => cancelAnimationFrame(raf);
    }
    setEntered(false);
    const timeout = window.setTimeout(() => setMounted(false), LYRICS_TRANSITION_MS);
    return () => window.clearTimeout(timeout);
  }, [showLyrics]);

  return { mounted, entered };
}

interface FullscreenPlayerProps {
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
  lyrics: Lyrics | null;
  lyricsLoading: boolean;
  activeLineIndex: number;
  onToggleLyrics: () => void;
  onClose: () => void;
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
}

export function FullscreenPlayer({
  currentSong, currentCoverSrc, currentTime, isPlaying, isShuffle, repeatMode,
  isNormalizeVolume, isMuted, volume, showLyrics, lyrics, lyricsLoading, activeLineIndex,
  onToggleLyrics, onClose,
  onToggleShuffle, onPrev, onTogglePlay, onNext, onToggleRepeat,
  onSeekDragStart, onSeekScrub, onSeekCommit,
  onToggleNormalize, onToggleMute, onVolumeChange, onVolumeWheel,
}: FullscreenPlayerProps) {
  const { mounted: lyricsMounted, entered: lyricsEntered } = useLyricsColumnTransition(showLyrics);
  const spectrumRef = useSpectrumData();
  const [visualizerMode, setVisualizerMode] = useState<VisualizerMode>("bars");

  const cycleVisualizerMode = () => {
    const idx = VISUALIZER_MODES.indexOf(visualizerMode);
    setVisualizerMode(VISUALIZER_MODES[(idx + 1) % VISUALIZER_MODES.length]);
  };

  return (
    <div className="fullscreen-overlay">
      <VisualizerBackground mode={visualizerMode} dataRef={spectrumRef} />

      <div className="fullscreen-header">
        <span className="fullscreen-title-label">REPRODUCIENDO AHORA</span>
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            className="btn-icon"
            onClick={cycleVisualizerMode}
            title={`Visualizador: ${VISUALIZER_LABELS[visualizerMode]} (clic para cambiar)`}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="white"><path d="M7 14H5v5h2v-5zm4-9H9v14h2V5zm4 4h-2v10h2V9zm4-6h-2v16h2V3z" /></svg>
          </button>
          <button
            className={`btn-icon ${showLyrics ? "active" : ""}`}
            onClick={onToggleLyrics}
            title={showLyrics ? "Ocultar letra" : "Ver letra"}
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="white"><path d="M14 17H4v2h10v-2zm6-8H4v2h16V9zM4 15h16v-2H4v2zM4 5v2h16V5H4z" /></svg>
          </button>
          <button className="btn-icon" onClick={onClose}>
            <svg width="24" height="24" viewBox="0 0 24 24" fill="white"><path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z" /></svg>
          </button>
        </div>
      </div>

      <div className={`fullscreen-body ${lyricsMounted ? "with-lyrics" : ""}`}>
        <div className="fullscreen-track-column">
          {currentCoverSrc
            ? <img src={currentCoverSrc} alt="Portada" className="fullscreen-cover-large" />
            : <div className="fullscreen-cover-placeholder">🎧</div>
          }
          <div className="fullscreen-info">
            <h1 className="fullscreen-track-title">{currentSong?.title ?? "Sin canción"}</h1>
            <p className="fullscreen-track-artist">{currentSong?.artist ?? ""}</p>
            <p className="fullscreen-track-album">{currentSong?.album ?? ""}</p>
          </div>
        </div>

        {lyricsMounted && (
          <div className={`fullscreen-lyrics-column ${lyricsEntered ? "visible" : ""}`}>
            <LyricsPanel lyrics={lyrics} loading={lyricsLoading} activeLineIndex={activeLineIndex} />
          </div>
        )}
      </div>

      <div className="fullscreen-controls">
        <ProgressBar
          currentTime={currentTime}
          duration={currentSong?.duration_secs ?? 0}
          onDragStart={onSeekDragStart}
          onScrub={onSeekScrub}
          onCommit={onSeekCommit}
        />

        <TransportButtons
          size="large"
          isShuffle={isShuffle}
          isPlaying={isPlaying}
          repeatMode={repeatMode}
          onToggleShuffle={onToggleShuffle}
          onPrev={onPrev}
          onTogglePlay={onTogglePlay}
          onNext={onNext}
          onToggleRepeat={onToggleRepeat}
        />

        <VolumeControl
          size="large"
          isNormalizeVolume={isNormalizeVolume}
          onToggleNormalize={onToggleNormalize}
          isMuted={isMuted}
          volume={volume}
          onToggleMute={onToggleMute}
          onVolumeChange={onVolumeChange}
          onWheel={onVolumeWheel}
        />
      </div>
    </div>
  );
}
