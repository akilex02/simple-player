import { memo, useEffect, useRef } from "react";
import type { Lyrics } from "../hooks/useLyrics";

interface LyricsPanelProps {
  lyrics: Lyrics | null;
  loading: boolean;
  activeLineIndex: number;
}

export const LyricsPanel = memo(function LyricsPanel({ lyrics, loading, activeLineIndex }: LyricsPanelProps) {
  const activeLineRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    activeLineRef.current?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [activeLineIndex]);

  if (loading) {
    return <div className="lyrics-panel lyrics-empty">Buscando letra...</div>;
  }

  if (!lyrics || lyrics.lines.length === 0) {
    return <div className="lyrics-panel lyrics-empty">No hay letra disponible para esta canción</div>;
  }

  return (
    <div className="lyrics-panel">
      {lyrics.lines.map((line, i) => {
        const text = typeof line === "string" ? line : line.text;
        const isActive = lyrics.kind === "Synced" && i === activeLineIndex;
        return (
          <p
            key={i}
            ref={isActive ? activeLineRef : null}
            className={`lyrics-line ${isActive ? "active" : ""}`}
          >
            {text || " "}
          </p>
        );
      })}
    </div>
  );
});
