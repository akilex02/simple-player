import { memo } from "react";
import type { RepeatMode } from "../types";

interface TransportButtonsProps {
  size: "compact" | "large";
  isShuffle: boolean;
  isPlaying: boolean;
  repeatMode: RepeatMode;
  onToggleShuffle: () => void;
  onPrev: () => void;
  onTogglePlay: () => void;
  onNext: () => void;
  onToggleRepeat: () => void;
}

export const TransportButtons = memo(function TransportButtons({
  size, isShuffle, isPlaying, repeatMode,
  onToggleShuffle, onPrev, onTogglePlay, onNext, onToggleRepeat,
}: TransportButtonsProps) {
  const sideIconSize = size === "large" ? 24 : 20;
  const smallIconSize = size === "large" ? 22 : 18;
  const playIconSize = size === "large" ? 28 : 22;
  const playButtonStyle = size === "large" ? { width: "56px", height: "56px" } : undefined;

  return (
    <div
      className="control-buttons"
      style={size === "large" ? { justifyContent: "center" } : undefined}
    >
      <button className={`btn-icon ${isShuffle ? "active" : ""}`} onClick={onToggleShuffle} title={isShuffle ? "Aleatorio: Activado" : "Aleatorio: Desactivado"}>
        <svg width={smallIconSize} height={smallIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.45 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" /></svg>
      </button>

      <button className="btn-icon" onClick={onPrev}>
        <svg width={sideIconSize} height={sideIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" /></svg>
      </button>

      <button className="btn-play-main" onClick={onTogglePlay} style={playButtonStyle}>
        {isPlaying
          ? <svg width={playIconSize} height={playIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" /></svg>
          : <svg width={playIconSize} height={playIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
        }
      </button>

      <button className="btn-icon" onClick={onNext}>
        <svg width={sideIconSize} height={sideIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" /></svg>
      </button>

      <button className={`btn-icon ${repeatMode !== "off" ? "active" : ""}`} onClick={onToggleRepeat} title={`Repetir: ${repeatMode === "off" ? "Desactivado" : repeatMode === "all" ? "Todo" : "Una canción"}`}>
        <svg width={smallIconSize} height={smallIconSize} viewBox="0 0 24 24" fill="currentColor"><path d="M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v3z" /></svg>
        {repeatMode === "one" && <span className="repeat-badge">1</span>}
      </button>
    </div>
  );
});
