import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface SyncedLine {
  time_ms: number;
  text: string;
}

export type Lyrics =
  | { kind: "Synced"; lines: SyncedLine[] }
  | { kind: "Plain"; lines: string[] };

export function useLyrics(songPath: string | null, currentTimeSecs: number) {
  const [lyrics, setLyrics] = useState<Lyrics | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!songPath) {
      setLyrics(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    invoke<Lyrics | null>("get_lyrics", { songPath })
      .then(result => { if (!cancelled) setLyrics(result); })
      .catch(() => { if (!cancelled) setLyrics(null); })
      .finally(() => { if (!cancelled) setLoading(false); });

    return () => { cancelled = true; };
  }, [songPath]);

  const activeLineIndex = useMemo(() => {
    if (!lyrics || lyrics.kind !== "Synced") return -1;
    const timeMs = currentTimeSecs * 1000;
    let idx = -1;
    for (let i = 0; i < lyrics.lines.length; i++) {
      if (lyrics.lines[i].time_ms <= timeMs) idx = i;
      else break;
    }
    return idx;
  }, [lyrics, currentTimeSecs]);

  return { lyrics, loading, activeLineIndex };
}
