import {
  useState, useEffect, useMemo, useDeferredValue, memo,
  useCallback, useRef, startTransition
} from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

// Convert absolute cover path to Tauri asset:// URL (WebKitGTK trusted origin)
function getCoverUrl(coverPath: string | null): string | null {
  if (!coverPath) return null;
  // Already a data: URL - return as-is
  if (coverPath.startsWith('data:')) return coverPath;
  // Absolute file path -> convertFileSrc turns it into asset://localhost/...
  return convertFileSrc(coverPath);
}

interface Song {
  path: string;
  title: string;
  artist: string;
  album: string;
  duration_secs: number;
  cover_art: string | null;
}

interface PlaybackState {
  folder_path: string | null;
  queue_paths: string[];
  current_index: number | null;
  volume: number;
  is_shuffle: boolean;
  repeat_mode: string;
}

type SortField = "title" | "artist" | "album" | "duration";
type ActiveTab = "all" | "artists";
type RepeatMode = "off" | "all" | "one";

function shuffleArray<T>(array: T[]): T[] {
  const arr = [...array];
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

// Memoized SongRow - coverSrc is pre-resolved
const SongRow = memo(
  ({
    song, isCurrent, coverSrc, onPlay, formatTime,
  }: {
    song: Song; isCurrent: boolean; coverSrc: string | null;
    onPlay: (song: Song) => void; formatTime: (secs: number) => string;
  }) => (
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
  )
);

export function App() {
  const [songs, setSongs] = useState<Song[]>([]);
  const [currentSongIndex, setCurrentSongIndex] = useState<number | null>(null);
  const [activeQueue, setActiveQueue] = useState<Song[]>([]);
  const [isPlaying, setIsPlaying] = useState(false);
  const [volume, setVolume] = useState(0.8);
  const [lastVolume, setLastVolume] = useState(0.8);
  const [isMuted, setIsMuted] = useState(false);

  const [searchQuery, setSearchQuery] = useState("");
  const deferredSearch = useDeferredValue(searchQuery);

  const [currentTime, setCurrentTime] = useState(0);
  const [isDraggingSeek, setIsDraggingSeek] = useState(false);
  const [loading, setLoading] = useState(false);
  const [currentFolderPath, setCurrentFolderPath] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<ActiveTab>("all");
  const [selectedArtist, setSelectedArtist] = useState<string | null>(null);

  const [sortField, setSortField] = useState<SortField>("title");
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("asc");
  const [artistSortOrder, setArtistSortOrder] = useState<"asc" | "desc">("asc");

  const [isShuffle, setIsShuffle] = useState(false);
  const [repeatMode, setRepeatMode] = useState<RepeatMode>("off");
  const [isNormalizeVolume, setIsNormalizeVolume] = useState(true);

  const [isFullscreen, setIsFullscreen] = useState(false);

  // Ref to always have latest handlers in keyboard listener without re-registering
  const togglePlayPauseRef = useRef<() => void>(() => { });
  const handleNextSongRef = useRef<() => void>(() => { });
  const handlePrevSongRef = useRef<() => void>(() => { });
  const handleAutoNextSongRef = useRef<() => void>(() => { });

  // ─── Persist & Restore State ─────────────────────────────────────────────────

  const persistState = useCallback((
    queue: Song[], index: number | null, folder: string | null,
    vol: number, shuffle: boolean, repeat: RepeatMode
  ) => {
    invoke("save_playback_state", {
      state: {
        folder_path: folder,
        queue_paths: queue.map(s => s.path),
        current_index: index,
        volume: vol,
        is_shuffle: shuffle,
        repeat_mode: repeat,
      } as PlaybackState
    }).catch(() => { });
  }, []);

  // ─── Startup: load songs then restore state ──────────────────────────────────

  useEffect(() => {
    const init = async () => {
      setLoading(true);
      try {
        // 1. Load songs from cache (fast)
        const result = await invoke<Song[]>("scan_music_folder", { folderPath: null });
        const loadedSongs = result || [];
        setSongs(loadedSongs);

        // 2. Load persisted playback state
        const saved = await invoke<PlaybackState | null>("load_playback_state");
        if (saved && loadedSongs.length > 0) {
          if (saved.folder_path) setCurrentFolderPath(saved.folder_path);
          if (saved.is_shuffle !== undefined) setIsShuffle(saved.is_shuffle);
          if (saved.repeat_mode) setRepeatMode(saved.repeat_mode as RepeatMode);

          // Rebuild queue from saved paths
          if (saved.queue_paths && saved.queue_paths.length > 0) {
            const pathSet = new Map(loadedSongs.map(s => [s.path, s]));
            const restoredQueue = saved.queue_paths
              .map(p => pathSet.get(p))
              .filter(Boolean) as Song[];

            if (restoredQueue.length > 0) {
              setActiveQueue(restoredQueue);
              const restoredIndex = saved.current_index !== null &&
                saved.current_index < restoredQueue.length
                ? saved.current_index : 0;
              setCurrentSongIndex(restoredIndex);
            }
          }

          if (saved.volume !== undefined) {
            const vol = saved.volume;
            setVolume(vol);
            await invoke("set_volume", { volume: vol }).catch(() => {});
          }
        }
      } catch (err) {
        console.error("Error al inicializar:", err);
      } finally {
        setLoading(false);
      }
    };

    init();
  }, []);

  // ─── Keyboard Shortcuts ───────────────────────────────────────────────────────

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
      if (tag === 'input' || tag === 'textarea') return;

      if (e.code === 'Space') {
        e.preventDefault();
        togglePlayPauseRef.current();
      } else if (e.code === 'MediaPlayPause') {
        e.preventDefault();
        togglePlayPauseRef.current();
      } else if (e.code === 'MediaTrackNext' || e.code === 'ArrowRight' && e.altKey) {
        e.preventDefault();
        handleNextSongRef.current();
      } else if (e.code === 'MediaTrackPrevious' || e.code === 'ArrowLeft' && e.altKey) {
        e.preventDefault();
        handlePrevSongRef.current();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  // ─── Global Media Key Events (from Rust, work when app is minimized) ─────────

  useEffect(() => {
    const unlistenPrev = listen('media-prev', () => handlePrevSongRef.current());
    const unlistenNext = listen('media-next', () => handleNextSongRef.current());
    const unlistenPlay = listen('media-playpause', () => {
      setIsPlaying(prev => !prev);
    });
    const unlistenPlaying = listen<boolean>('media-playing', (event) => {
      setIsPlaying(event.payload);
    });

    return () => {
      unlistenPrev.then(fn => fn());
      unlistenNext.then(fn => fn());
      unlistenPlay.then(fn => fn());
      unlistenPlaying.then(fn => fn());
    };
  }, []);

  // ─── MPRIS2 Now-Playing Sync ─────────────────────────────────────────────────

  useEffect(() => {
    const song = currentSongIndex !== null && activeQueue[currentSongIndex]
      ? activeQueue[currentSongIndex] : null;
    if (!song) return;

    invoke("update_now_playing", {
      title: song.title,
      artist: song.artist,
      album: song.album,
      coverPath: song.cover_art ?? null,
      durationSecs: song.duration_secs,
      positionSecs: currentTime,
      isPlaying,
    }).catch(() => { });
  }, [currentSongIndex, isPlaying]);

  // ─── Position Polling ─────────────────────────────────────────────────────────

  useEffect(() => {
    let interval: ReturnType<typeof setInterval>;
    if (isPlaying && currentSongIndex !== null) {
      interval = setInterval(async () => {
        if (!isDraggingSeek) {
          try {
            const pos = await invoke<number>("get_position");
            setCurrentTime(pos);
            const q = activeQueue.length > 0 ? activeQueue : [];
            const cur = q[currentSongIndex];
            if (cur && cur.duration_secs > 0 && pos >= cur.duration_secs) {
              handleAutoNextSongRef.current();
            }
          } catch (_) { }
        }
      }, 500);
    }
    return () => clearInterval(interval);
  }, [isPlaying, currentSongIndex, activeQueue, repeatMode, isShuffle, isDraggingSeek]);

  // ─── Scan folder ─────────────────────────────────────────────────────────────

  const scanFolder = async (folder: string | null) => {
    setLoading(true);
    try {
      const result = await invoke<Song[]>("scan_music_folder", { folderPath: folder });
      setSongs(result || []);
    } catch (err) {
      console.error("Error al escanear:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleSelectFolder = async () => {
    try {
      const folder = await invoke<string | null>("select_folder");
      if (folder) {
        setCurrentFolderPath(folder);
        scanFolder(folder);
      }
    } catch (err) {
      console.error("Error al seleccionar carpeta:", err);
    }
  };

  // ─── Playback ─────────────────────────────────────────────────────────────────

  const playIndex = async (index: number, queueToUse?: Song[]) => {
    const queue = queueToUse || activeQueue;
    if (queue.length === 0) return;

    const validIndex = (index + queue.length) % queue.length;
    const song = queue[validIndex];
    if (!song) return;

    if (queueToUse) {
      setActiveQueue(queueToUse);
    }
    setCurrentSongIndex(validIndex);
    setCurrentTime(0);

    try {
      await invoke("play_song", { songPath: song.path });
      setIsPlaying(true);
    } catch (err) {
      console.error("Error al reproducir:", err);
    }

    persistState(queue, validIndex, currentFolderPath, volume, isShuffle, repeatMode);
  };

  const handlePlaySongFromList = (song: Song, sourceQueue?: Song[]) => {
    const baseQueue = sourceQueue || (activeQueue.length > 0 ? activeQueue : sortedSongs);
    if (baseQueue.length === 0) return;

    if (isShuffle) {
      const others = baseQueue.filter(s => s.path !== song.path);
      const newShuffledQueue = [song, ...shuffleArray(others)];
      playIndex(0, newShuffledQueue);
    } else {
      const idx = baseQueue.findIndex(s => s.path === song.path);
      playIndex(idx !== -1 ? idx : 0, baseQueue);
    }
  };

  const startShufflePlay = (songList: Song[]) => {
    if (songList.length === 0) return;
    const shuffled = shuffleArray(songList);
    setIsShuffle(true);
    playIndex(0, shuffled);
  };

  const toggleShuffle = () => {
    const curSong = currentSongIndex !== null && activeQueue[currentSongIndex]
      ? activeQueue[currentSongIndex]
      : null;

    if (!isShuffle) {
      const baseList = activeQueue.length > 0 ? activeQueue : sortedSongs;
      if (baseList.length === 0) return;
      let shuffled: Song[];
      if (curSong) {
        const others = baseList.filter(s => s.path !== curSong.path);
        shuffled = [curSong, ...shuffleArray(others)];
      } else {
        shuffled = shuffleArray(baseList);
      }
      setIsShuffle(true);
      setActiveQueue(shuffled);
      setCurrentSongIndex(0);
      persistState(shuffled, 0, currentFolderPath, volume, true, repeatMode);
    } else {
      const baseList = sortedSongs;
      setIsShuffle(false);
      const newIdx = curSong ? baseList.findIndex(s => s.path === curSong.path) : 0;
      const idx = newIdx !== -1 ? newIdx : 0;
      setActiveQueue(baseList);
      setCurrentSongIndex(idx);
      persistState(baseList, idx, currentFolderPath, volume, false, repeatMode);
    }
  };

  const togglePlayPause = async () => {
    if (currentSongIndex === null) {
      if (activeQueue.length > 0) playIndex(0);
      else if (sortedSongs.length > 0) handlePlaySongFromList(sortedSongs[0], sortedSongs);
      return;
    }
    try {
      if (isPlaying) {
        await invoke("pause_song").catch(() => {});
        setIsPlaying(false);
      } else {
        const song = activeQueue[currentSongIndex];
        if (song) {
          try {
            const pos = await invoke<number>("get_position");
            if (pos === 0) {
              await invoke("play_song", { songPath: song.path });
            } else {
              await invoke("resume_song");
            }
          } catch {
            await invoke("play_song", { songPath: song.path });
          }
        }
        setIsPlaying(true);
      }
    } catch (err) {
      console.error("Error al cambiar reproducción:", err);
    }
  };

  const handleAutoNextSong = () => {
    const queue = activeQueue.length > 0 ? activeQueue : sortedSongs;
    if (queue.length === 0) return;
    const curIdx = currentSongIndex ?? 0;
    if (repeatMode === "one") {
      playIndex(curIdx);
      return;
    }
    if (repeatMode === "off" && curIdx === queue.length - 1) {
      setIsPlaying(false);
      return;
    }
    playIndex(curIdx + 1);
  };

  const handleNextSong = () => {
    const queue = activeQueue.length > 0 ? activeQueue : sortedSongs;
    if (queue.length === 0) return;
    const curIdx = currentSongIndex ?? 0;
    playIndex(curIdx + 1);
  };

  const handlePrevSong = () => {
    const queue = activeQueue.length > 0 ? activeQueue : sortedSongs;
    if (queue.length === 0) return;
    const curIdx = currentSongIndex ?? 0;
    playIndex(curIdx - 1);
  };

  // Update refs so keyboard shortcuts & event handlers always call the latest function
  useEffect(() => { togglePlayPauseRef.current = togglePlayPause; });
  useEffect(() => { handleNextSongRef.current = handleNextSong; });
  useEffect(() => { handlePrevSongRef.current = handlePrevSong; });
  useEffect(() => { handleAutoNextSongRef.current = handleAutoNextSong; });

  const handleSeekCommit = async (newSecs: number) => {
    setIsDraggingSeek(false);
    setCurrentTime(newSecs);
    try {
      await invoke("seek_song", { positionSecs: newSecs });
    } catch (err) {
      console.error("Error al adelantar:", err);
    }
  };

  const toggleMute = async () => {
    if (isMuted) {
      setIsMuted(false);
      setVolume(lastVolume);
      await invoke("set_volume", { volume: lastVolume }).catch(console.error);
    } else {
      setLastVolume(volume);
      setIsMuted(true);
      setVolume(0);
      await invoke("set_volume", { volume: 0 }).catch(console.error);
    }
  };

  const handleVolumeChange = async (newVol: number) => {
    setVolume(newVol);
    if (newVol > 0) setIsMuted(false);
    await invoke("set_volume", { volume: newVol }).catch(console.error);
  };

  const handleVolumeWheel = (e: React.WheelEvent) => {
    const step = 0.02;
    const delta = e.deltaY < 0 ? step : -step;
    const newVol = Math.min(1, Math.max(0, Math.round((volume + delta) * 100) / 100));
    handleVolumeChange(newVol);
  };

  const toggleRepeatMode = () => {
    const next: RepeatMode = repeatMode === "off" ? "all" : repeatMode === "all" ? "one" : "off";
    setRepeatMode(next);
    persistState(activeQueue, currentSongIndex, currentFolderPath, volume, isShuffle, next);
  };

  const toggleNormalizeVolume = async () => {
    const next = !isNormalizeVolume;
    setIsNormalizeVolume(next);
    await invoke("set_audio_normalization", { enabled: next }).catch(console.error);
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) setSortDirection(prev => prev === "asc" ? "desc" : "asc");
    else { setSortField(field); setSortDirection("asc"); }
  };

  const renderSortIndicator = (field: SortField) => {
    if (sortField !== field) return null;
    return sortDirection === "asc" ? " ▲" : " ▼";
  };

  const formatTime = (seconds: number) => {
    if (isNaN(seconds)) return "0:00";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
  };

  // ─── Derived Data ─────────────────────────────────────────────────────────────

  // Strawberry-style tokenized search with relevance scoring
  const filteredSongs = useMemo(() => {
    const query = deferredSearch.trim().toLowerCase();
    let base = selectedArtist ? songs.filter(s => s.artist === selectedArtist) : songs;
    if (!query) return base;

    const tokens = query.split(/\s+/).filter(Boolean);
    return base
      .map(s => {
        const titleL = s.title.toLowerCase();
        const artistL = s.artist.toLowerCase();
        const albumL = s.album.toLowerCase();
        let score = 0;
        for (const tok of tokens) {
          if (titleL.startsWith(tok)) score += 10;
          else if (titleL.includes(` ${tok}`)) score += 8;
          else if (titleL.includes(tok)) score += 5;
          if (artistL.startsWith(tok)) score += 9;
          else if (artistL.includes(tok)) score += 4;
          if (albumL.startsWith(tok)) score += 7;
          else if (albumL.includes(tok)) score += 3;
        }
        const allMatch = tokens.every(tok =>
          titleL.includes(tok) || artistL.includes(tok) || albumL.includes(tok)
        );
        return { song: s, score: allMatch ? score : 0 };
      })
      .filter(x => x.score > 0)
      .sort((a, b) => b.score - a.score)
      .map(x => x.song);
  }, [songs, deferredSearch, selectedArtist]);

  const sortedSongs = useMemo(() => {
    let result: Song[];
    if (deferredSearch) {
      result = filteredSongs; // Already sorted by relevance
    } else {
      result = [...filteredSongs].sort((a, b) => {
        let vA: string | number = "", vB: string | number = "";
        if (sortField === "title") { vA = a.title.toLowerCase(); vB = b.title.toLowerCase(); }
        else if (sortField === "artist") { vA = a.artist.toLowerCase(); vB = b.artist.toLowerCase(); }
        else if (sortField === "album") { vA = a.album.toLowerCase(); vB = b.album.toLowerCase(); }
        else if (sortField === "duration") { vA = a.duration_secs; vB = b.duration_secs; }
        if (vA < vB) return sortDirection === "asc" ? -1 : 1;
        if (vA > vB) return sortDirection === "asc" ? 1 : -1;
        return 0;
      });
    }
    return result;
  }, [filteredSongs, sortField, sortDirection, deferredSearch]);

  const artistGroups = useMemo(() => {
    const map = new Map<string, Song[]>();
    const query = deferredSearch.trim().toLowerCase();
    const tokens = query.split(/\s+/).filter(Boolean);

    let songsToGroup = songs;
    if (tokens.length > 0) {
      songsToGroup = songs.filter(s => {
        const titleL = s.title.toLowerCase();
        const artistL = s.artist.toLowerCase();
        const albumL = s.album.toLowerCase();
        return tokens.every(tok =>
          titleL.includes(tok) || artistL.includes(tok) || albumL.includes(tok)
        );
      });
    }

    for (const song of songsToGroup) {
      const artist = song.artist || "Artista Desconocido";
      if (!map.has(artist)) map.set(artist, []);
      map.get(artist)!.push(song);
    }

    const result = Array.from(map.entries())
      .sort((a, b) => {
        const cmp = a[0].localeCompare(b[0], undefined, { sensitivity: "base" });
        return artistSortOrder === "asc" ? cmp : -cmp;
      })
      .map(([artist, artistSongs]) => ({
        artist,
        count: artistSongs.length,
        representativeSong: artistSongs.find(s => s.cover_art !== null) || null,
      }));

    return result;
  }, [songs, deferredSearch, artistSortOrder]);

  const currentSong = currentSongIndex !== null && activeQueue[currentSongIndex]
    ? activeQueue[currentSongIndex] : null;

  const currentCoverSrc = getCoverUrl(currentSong?.cover_art ?? null);

  const progressPct = currentSong ? (currentTime / currentSong.duration_secs) * 100 : 0;
  const volPct = volume * 100;

  // ─── JSX Helpers ─────────────────────────────────────────────────────────────

  const seekSliderProps = {
    min: "0",
    max: currentSong ? currentSong.duration_secs : 100,
    value: currentTime,
    style: { "--pct": `${progressPct}%` } as React.CSSProperties,
    onMouseDown: () => setIsDraggingSeek(true),
    onTouchStart: () => setIsDraggingSeek(true),
    onChange: (e: React.ChangeEvent<HTMLInputElement>) => setCurrentTime(Number(e.target.value)),
    onMouseUp: (e: React.MouseEvent<HTMLInputElement>) => handleSeekCommit(Number((e.target as HTMLInputElement).value)),
    onTouchEnd: (e: React.TouchEvent<HTMLInputElement>) => handleSeekCommit(Number((e.target as HTMLInputElement).value)),
  };

  const VolumeIcon = () => (isMuted || volume === 0)
    ? <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M16.5 12c0-1.77-1.02-3.29-2.5-4.03v2.21l2.45 2.45c.03-.2.05-.41.05-.63zm2.5 0c0 .94-.2 1.82-.54 2.64l1.51 1.51C20.63 14.91 21 13.5 21 12c0-4.28-2.99-7.86-7-8.77v2.06c2.89.86 5 3.54 5 6.71zM4.27 3L3 4.27 7.73 9H3v6h4l5 5v-6.73l4.25 4.25c-.67.52-1.42.93-2.25 1.18v2.06c1.38-.31 2.63-.95 3.69-1.81L19.73 21 21 19.73 4.27 3zM12 4L9.91 6.09 12 8.18V4z" /></svg>
    : <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z" /></svg>;

  // ─── Render ───────────────────────────────────────────────────────────────────

  return (
    <div className="root-app">
      <div className="app-container">
        {/* Sidebar */}
        <aside className="sidebar">
          <div className="brand">
            <img
              src="/app-icon.png"
              alt="Simple Player"
              className="brand-logo-img"
            />
            <span className="brand-name">Simple Player</span>
          </div>

          <div className="nav-section">
            <span className="nav-title">Navegación</span>
            <div
              className={`nav-item ${activeTab === "all" ? "active" : ""}`}
              onClick={() => startTransition(() => { setActiveTab("all"); setSelectedArtist(null); })}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3L1 9l11 6 9-4.91V17h2V9L12 3zM3.27 9L12 4.24 20.73 9 12 13.76 3.27 9z" /></svg>
              <span>Toda la Música</span>
            </div>

            <div
              className={`nav-item ${activeTab === "artists" ? "active" : ""}`}
              onClick={() => startTransition(() => { setActiveTab("artists"); setSelectedArtist(null); })}
            >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z" /></svg>
              <span>Artistas</span>
            </div>
          </div>
        </aside>

        {/* Main Content */}
        <main className="main-content">
          <header className="header">
            <div className="search-box">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="#9ca3af"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" /></svg>
              <input
                type="text"
                placeholder="Buscar canciones, artistas..."
                value={searchQuery}
                onChange={e => startTransition(() => setSearchQuery(e.target.value))}
              />
              {searchQuery && (
                <button
                  style={{ background: "none", border: "none", color: "#9ca3af", cursor: "pointer", padding: "0 4px", fontSize: "16px", lineHeight: 1 }}
                  onClick={() => startTransition(() => setSearchQuery(""))}
                  title="Borrar búsqueda"
                >✕</button>
              )}
            </div>

            <button className="btn-scan" onClick={handleSelectFolder} disabled={loading}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="white"><path d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z" /></svg>
              <span>{loading ? "Escaneando..." : currentFolderPath ? "Cambiar carpeta..." : "Abrir carpeta de música"}</span>
            </button>
          </header>

          <div className="song-list-container">
            {activeTab === "artists" && !selectedArtist ? (
              <>
                <div className="artists-header-bar">
                  <span className="artists-header-title">
                    Artistas ({artistGroups.length})
                  </span>
                  <button
                    className="btn-sort-artist"
                    onClick={() => setArtistSortOrder(p => p === "asc" ? "desc" : "asc")}
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
                    const repSong = item.representativeSong;
                    const coverSrc = getCoverUrl(repSong?.cover_art ?? null);
                    return (
                      <div key={item.artist} className="artist-card" onClick={() => startTransition(() => setSelectedArtist(item.artist))}>
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
            ) : (
              <>
                {selectedArtist && (
                  <div className="artist-view-header">
                    <button className="btn-back" onClick={() => startTransition(() => setSelectedArtist(null))}>
                      ← Volver a Artistas
                    </button>
                    <h2>{selectedArtist}</h2>
                  </div>
                )}

                {sortedSongs.length > 0 && (
                  <button className="btn-shuffle-hero" onClick={() => startShufflePlay(sortedSongs)}>
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="white"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.45 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" /></svg>
                    <span>{selectedArtist ? `Reproducción Aleatoria de ${selectedArtist}` : "Reproducción Aleatoria de toda la música"}</span>
                  </button>
                )}

                {sortedSongs.length > 0 ? (
                  <>
                    <div className="song-list-header">
                      <span>#</span>
                      <span className="sort-header-btn" onClick={() => handleSort("title")}>Título{renderSortIndicator("title")}</span>
                      <span className="sort-header-btn" onClick={() => handleSort("artist")}>Artista{renderSortIndicator("artist")}</span>
                      <span className="sort-header-btn" onClick={() => handleSort("album")}>Álbum{renderSortIndicator("album")}</span>
                      <span className="sort-header-btn" onClick={() => handleSort("duration")}>Duración{renderSortIndicator("duration")}</span>
                    </div>
                    {sortedSongs.map(song => {
                      const isCurrent = currentSong !== null && currentSong.path === song.path;
                      const coverSrc = getCoverUrl(song.cover_art);
                      return (
                        <SongRow
                          key={song.path}
                          song={song}
                          isCurrent={isCurrent}
                          coverSrc={coverSrc}
                          onPlay={s => handlePlaySongFromList(s, sortedSongs)}
                          formatTime={formatTime}
                        />
                      );
                    })}
                  </>
                ) : (
                  <div className="empty-state">
                    <div className="empty-state-icon">🎵</div>
                    <p>{currentFolderPath ? `No hay canciones en "${currentFolderPath}"` : "No se encontraron canciones"}</p>
                    <button className="btn-scan" onClick={handleSelectFolder}>Seleccionar carpeta de música</button>
                  </div>
                )}
              </>
            )}
          </div>
        </main>
      </div>

      {/* Player Bar */}
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
          <div className="control-buttons">
            <button className={`btn-icon ${isShuffle ? "active" : ""}`} onClick={toggleShuffle} title={isShuffle ? "Aleatorio: Activado" : "Aleatorio: Desactivado"}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.45 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" /></svg>
            </button>

            <button className="btn-icon" onClick={handlePrevSong}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" /></svg>
            </button>

            <button className="btn-play-main" onClick={togglePlayPause}>
              {isPlaying
                ? <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" /></svg>
                : <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
              }
            </button>

            <button className="btn-icon" onClick={handleNextSong}>
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" /></svg>
            </button>

            <button className={`btn-icon ${repeatMode !== "off" ? "active" : ""}`} onClick={toggleRepeatMode} title={`Repetir: ${repeatMode === "off" ? "Desactivado" : repeatMode === "all" ? "Todo" : "Una canción"}`}>
              <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v3z" /></svg>
              {repeatMode === "one" && <span className="repeat-badge">1</span>}
            </button>
          </div>

          <div className="progress-bar-box">
            <span>{formatTime(currentTime)}</span>
            <input type="range" className="progress-slider" {...seekSliderProps} />
            <span>{currentSong ? formatTime(currentSong.duration_secs) : "0:00"}</span>
          </div>
        </div>

        <div className="player-extra-controls">
          <button
            className={`btn-icon ${isNormalizeVolume ? "active" : ""}`}
            onClick={toggleNormalizeVolume}
            title={isNormalizeVolume ? "Normalización de Audio: ACTIVADA (Iguala el volumen entre canciones altas y bajas)" : "Normalización de Audio: DESACTIVADA"}
            style={{ fontSize: "0.7rem", fontWeight: 700, padding: "4px 7px", borderRadius: "6px", border: "1px solid var(--border-subtle)" }}
          >
            NORM
          </button>

          <div className="volume-box" onWheel={handleVolumeWheel}>
            <button className="btn-icon" onClick={toggleMute} title={isMuted ? "Desmutear" : "Mutear"} style={{ color: "#9ca3af" }}>
              <VolumeIcon />
            </button>
            <input type="range" className="volume-slider" min="0" max="1" step="0.05" value={volume}
              style={{ "--vol-pct": `${volPct}%` } as React.CSSProperties}
              onChange={e => handleVolumeChange(Number(e.target.value))} />
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
                  if (!isNaN(val)) {
                    handleVolumeChange(Math.min(100, Math.max(0, val)) / 100);
                  }
                }}
                onWheel={handleVolumeWheel}
              />
              <span className="volume-unit-label">%</span>
            </div>
          </div>

          <button className="btn-icon" onClick={() => setIsFullscreen(true)} title="Pantalla Completa">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M7 14H5v5h5v-2H7v-3zm-2-4h2V7h3V5H5v5zm12 7h-3v2h5v-5h-2v3zM14 5v2h3v3h2V5h-5z" /></svg>
          </button>
        </div>
      </div>

      {/* Fullscreen Overlay */}
      {isFullscreen && (
        <div className="fullscreen-overlay">
          {currentCoverSrc && <img src={currentCoverSrc} alt="Fondo" className="fullscreen-bg" />}

          <div className="fullscreen-header">
            <span className="fullscreen-title-label">REPRODUCIENDO AHORA</span>
            <button className="btn-icon" onClick={() => setIsFullscreen(false)}>
              <svg width="24" height="24" viewBox="0 0 24 24" fill="white"><path d="M5 16h3v3h2v-5H5v2zm3-8H5v2h5V5H8v3zm6 11h2v-3h3v-2h-5v5zm2-11V5h-2v5h5V8h-3z" /></svg>
            </button>
          </div>

          <div className="fullscreen-body">
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

          <div className="fullscreen-controls">
            <div className="progress-bar-box">
              <span>{formatTime(currentTime)}</span>
              <input type="range" className="progress-slider" {...seekSliderProps} />
              <span>{currentSong ? formatTime(currentSong.duration_secs) : "0:00"}</span>
            </div>

            <div className="control-buttons" style={{ justifyContent: "center", marginTop: "8px" }}>
              <button className={`btn-icon ${isShuffle ? "active" : ""}`} onClick={toggleShuffle}>
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><path d="M10.59 9.17L5.41 4 4 5.41l5.17 5.17 1.42-1.41zM14.5 4l2.04 2.04L4 18.59 5.41 20 17.96 7.45 20 9.5V4h-5.5zm.33 9.41l-1.41 1.41 3.13 3.13L14.5 20H20v-5.5l-2.04 2.04-3.13-3.13z" /></svg>
              </button>
              <button className="btn-icon" onClick={handlePrevSong}>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" /></svg>
              </button>
              <button className="btn-play-main" onClick={togglePlayPause} style={{ width: "56px", height: "56px" }}>
                {isPlaying
                  ? <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" /></svg>
                  : <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
                }
              </button>
              <button className="btn-icon" onClick={handleNextSong}>
                <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" /></svg>
              </button>
              <button className={`btn-icon ${repeatMode !== "off" ? "active" : ""}`} onClick={toggleRepeatMode}>
                <svg width="22" height="22" viewBox="0 0 24 24" fill="currentColor"><path d="M7 7h10v3l4-4-4-4v3H5v6h2V7zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2v3z" /></svg>
                {repeatMode === "one" && <span className="repeat-badge">1</span>}
              </button>
            </div>

            <div className="fullscreen-volume-box" onWheel={handleVolumeWheel}>
              <button
                className={`btn-icon ${isNormalizeVolume ? "active" : ""}`}
                onClick={toggleNormalizeVolume}
                title={isNormalizeVolume ? "Normalización de Audio: ACTIVADA" : "Normalización de Audio: DESACTIVADA"}
                style={{ fontSize: "0.7rem", fontWeight: 700, padding: "4px 7px", borderRadius: "6px", border: "1px solid var(--border-subtle)", marginRight: "8px" }}
              >
                NORM
              </button>
              <button className="btn-icon" onClick={toggleMute} style={{ color: "rgba(255,255,255,0.7)" }}>
                <VolumeIcon />
              </button>
              <input type="range" className="fullscreen-vol-slider" min="0" max="1" step="0.05" value={volume}
                style={{ "--vol-pct": `${volPct}%` } as React.CSSProperties}
                onChange={e => handleVolumeChange(Number(e.target.value))} />
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
                    if (!isNaN(val)) {
                      handleVolumeChange(Math.min(100, Math.max(0, val)) / 100);
                    }
                  }}
                  onWheel={handleVolumeWheel}
                />
                <span className="volume-unit-label">%</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
