import { memo } from "react";

interface VolumeControlProps {
  size: "compact" | "large";
  isNormalizeVolume: boolean;
  onToggleNormalize: () => void;
  isMuted: boolean;
  volume: number;
  onToggleMute: () => void;
  onVolumeChange: (vol: number) => void;
  onWheel: (e: React.WheelEvent) => void;
}

function VolumeIcon({ muted }: { muted: boolean }) {
  return muted
    ? <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73 4.27 3zM12 4L9.91 6.09 12 8.18V4z" /></svg>
    : <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z" /></svg>;
}

export const VolumeControl = memo(function VolumeControl({
  size, isNormalizeVolume, onToggleNormalize, isMuted, volume,
  onToggleMute, onVolumeChange, onWheel,
}: VolumeControlProps) {
  const volPct = volume * 100;
  const boxClassName = size === "large" ? "fullscreen-volume-box" : "volume-box";
  const sliderClassName = size === "large" ? "fullscreen-vol-slider" : "volume-slider";

  return (
    <div className={boxClassName} onWheel={onWheel}>
      <button
        className={`btn-icon ${isNormalizeVolume ? "active" : ""}`}
        onClick={onToggleNormalize}
        title={isNormalizeVolume ? "Normalización de Audio: ACTIVADA (Iguala el volumen entre canciones altas y bajas)" : "Normalización de Audio: DESACTIVADA"}
        style={{ fontSize: "0.7rem", fontWeight: 700, padding: "4px 7px", borderRadius: "6px", border: "1px solid var(--border-subtle)", marginRight: size === "large" ? "8px" : undefined }}
      >
        NORM
      </button>
      <button
        className="btn-icon"
        onClick={onToggleMute}
        title={isMuted ? "Desmutear" : "Mutear"}
        style={{ color: size === "large" ? "rgba(255,255,255,0.7)" : "#9ca3af" }}
      >
        <VolumeIcon muted={isMuted || volume === 0} />
      </button>
      <input
        type="range"
        className={sliderClassName}
        min="0"
        max="1"
        step="0.05"
        value={volume}
        style={{ "--vol-pct": `${volPct}%` } as React.CSSProperties}
        onChange={e => onVolumeChange(Number(e.target.value))}
      />
      <div className="volume-input-box" title="Usa la rueda del mouse o escribe un valor de 0 a 100%">
        <input
          type="number"
          className="volume-number-input"
          min="0"
          max="100"
          value={Math.round(volume * 100)}
          onChange={e => {
            const raw = e.target.value;
            if (raw === "") return;
            const val = parseInt(raw, 10);
            if (!isNaN(val)) onVolumeChange(Math.min(100, Math.max(0, val)) / 100);
          }}
          onWheel={onWheel}
        />
        <span className="volume-unit-label">%</span>
      </div>
    </div>
  );
});
