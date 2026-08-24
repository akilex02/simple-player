import { formatTime } from "../utils";

interface ProgressBarProps {
  currentTime: number;
  duration: number;
  onDragStart: () => void;
  onScrub: (secs: number) => void;
  onCommit: (secs: number) => void;
}

export function ProgressBar({ currentTime, duration, onDragStart, onScrub, onCommit }: ProgressBarProps) {
  const progressPct = duration > 0 ? (currentTime / duration) * 100 : 0;

  return (
    <div className="progress-bar-box">
      <span>{formatTime(currentTime)}</span>
      <input
        type="range"
        className="progress-slider"
        min="0"
        max={duration || 100}
        value={currentTime}
        style={{ "--pct": `${progressPct}%` } as React.CSSProperties}
        onMouseDown={onDragStart}
        onTouchStart={onDragStart}
        onChange={e => onScrub(Number(e.target.value))}
        onMouseUp={e => onCommit(Number((e.target as HTMLInputElement).value))}
        onTouchEnd={e => onCommit(Number((e.target as HTMLInputElement).value))}
      />
      <span>{duration > 0 ? formatTime(duration) : "0:00"}</span>
    </div>
  );
}
