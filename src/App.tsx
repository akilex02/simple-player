import {
  useState, useEffect, useMemo, useDeferredValue,
  useCallback, useRef
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useLyrics } from "./hooks/useLyrics";
import { AppHeader } from "./components/AppHeader";
import { ArtistsGrid } from "./components/ArtistsGrid";
import { FullscreenPlayer } from "./components/FullscreenPlayer";
import { PlayerBar } from "./components/PlayerBar";
import { Sidebar } from "./components/Sidebar";
import { SongTable } from "./components/SongTable";
import type { ActiveTab, PlaybackState, RepeatMode, Song, SortField } from "./types";
import { getCoverUrl, shuffleArray } from "./utils";
import "./App.css";

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
  const [showLyrics, setShowLyrics] = useState(false);

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

  // ─── Derived Data ─────────────────────────────────────────────────────────────
  // Se calcula antes de los handlers de abajo porque sus useCallback dependen de
  // sortedSongs (evaluado en cada render, a diferencia de un closure diferido).

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

  const { lyrics, loading: lyricsLoading, activeLineIndex } = useLyrics(
    currentSong?.path ?? null,
    currentTime
  );

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

  const scanFolder = useCallback(async (folder: string | null) => {
    setLoading(true);
    try {
      const result = await invoke<Song[]>("scan_music_folder", { folderPath: folder });
      setSongs(result || []);
    } catch (err) {
      console.error("Error al escanear:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const handleSelectFolder = useCallback(async () => {
    try {
      const folder = await invoke<string | null>("select_folder");
      if (folder) {
        setCurrentFolderPath(folder);
        scanFolder(folder);
      }
    } catch (err) {
      console.error("Error al seleccionar carpeta:", err);
    }
  }, [scanFolder]);

  // ─── Playback ─────────────────────────────────────────────────────────────────

  const playIndex = useCallback(async (index: number, queueToUse?: Song[]) => {
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
  }, [activeQueue, persistState, currentFolderPath, volume, isShuffle, repeatMode]);

  const handlePlaySongFromList = useCallback((song: Song, sourceQueue?: Song[]) => {
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
  }, [activeQueue, sortedSongs, isShuffle, playIndex]);

  const startShufflePlay = useCallback((songList: Song[]) => {
    if (songList.length === 0) return;
    const shuffled = shuffleArray(songList);
    setIsShuffle(true);
    playIndex(0, shuffled);
  }, [playIndex]);

  const toggleShuffle = useCallback(() => {
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
  }, [currentSongIndex, activeQueue, isShuffle, sortedSongs, persistState, currentFolderPath, volume, repeatMode]);

  const togglePlayPause = useCallback(async () => {
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
  }, [currentSongIndex, activeQueue, sortedSongs, handlePlaySongFromList, isPlaying, playIndex]);

  const handleAutoNextSong = useCallback(() => {
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
  }, [activeQueue, sortedSongs, currentSongIndex, repeatMode, playIndex]);

  const handleNextSong = useCallback(() => {
    const queue = activeQueue.length > 0 ? activeQueue : sortedSongs;
    if (queue.length === 0) return;
    const curIdx = currentSongIndex ?? 0;
    playIndex(curIdx + 1);
  }, [activeQueue, sortedSongs, currentSongIndex, playIndex]);

  const handlePrevSong = useCallback(() => {
    const queue = activeQueue.length > 0 ? activeQueue : sortedSongs;
    if (queue.length === 0) return;
    const curIdx = currentSongIndex ?? 0;
    playIndex(curIdx - 1);
  }, [activeQueue, sortedSongs, currentSongIndex, playIndex]);

  // Update refs so keyboard shortcuts & event handlers always call the latest function
  useEffect(() => { togglePlayPauseRef.current = togglePlayPause; });
  useEffect(() => { handleNextSongRef.current = handleNextSong; });
  useEffect(() => { handlePrevSongRef.current = handlePrevSong; });
  useEffect(() => { handleAutoNextSongRef.current = handleAutoNextSong; });

  const handleSeekCommit = useCallback(async (newSecs: number) => {
    setIsDraggingSeek(false);
    setCurrentTime(newSecs);
    try {
      await invoke("seek_song", { positionSecs: newSecs });
    } catch (err) {
      console.error("Error al adelantar:", err);
    }
  }, []);

  const toggleMute = useCallback(async () => {
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
  }, [isMuted, lastVolume, volume]);

  const handleVolumeChange = useCallback(async (newVol: number) => {
    setVolume(newVol);
    if (newVol > 0) setIsMuted(false);
    await invoke("set_volume", { volume: newVol }).catch(console.error);
  }, []);

  const handleVolumeWheel = useCallback((e: React.WheelEvent) => {
    const step = 0.02;
    const delta = e.deltaY < 0 ? step : -step;
    const newVol = Math.min(1, Math.max(0, Math.round((volume + delta) * 100) / 100));
    handleVolumeChange(newVol);
  }, [volume, handleVolumeChange]);

  const toggleRepeatMode = useCallback(() => {
    const next: RepeatMode = repeatMode === "off" ? "all" : repeatMode === "all" ? "one" : "off";
    setRepeatMode(next);
    persistState(activeQueue, currentSongIndex, currentFolderPath, volume, isShuffle, next);
  }, [repeatMode, persistState, activeQueue, currentSongIndex, currentFolderPath, volume, isShuffle]);

  const toggleNormalizeVolume = useCallback(async () => {
    const next = !isNormalizeVolume;
    setIsNormalizeVolume(next);
    await invoke("set_audio_normalization", { enabled: next }).catch(console.error);
  }, [isNormalizeVolume]);

  const handleSort = useCallback((field: SortField) => {
    if (sortField === field) setSortDirection(prev => prev === "asc" ? "desc" : "asc");
    else { setSortField(field); setSortDirection("asc"); }
  }, [sortField]);

  const handleSelectTab = useCallback((tab: ActiveTab) => {
    setActiveTab(tab);
    setSelectedArtist(null);
  }, []);

  const handleToggleArtistSortOrder = useCallback(() => {
    setArtistSortOrder(p => p === "asc" ? "desc" : "asc");
  }, []);

  const handleBackToArtists = useCallback(() => setSelectedArtist(null), []);
  const handleSeekDragStart = useCallback(() => setIsDraggingSeek(true), []);
  const handleOpenLyrics = useCallback(() => { setShowLyrics(true); setIsFullscreen(true); }, []);
  const handleOpenFullscreen = useCallback(() => setIsFullscreen(true), []);
  const handleToggleLyricsVisibility = useCallback(() => setShowLyrics(prev => !prev), []);
  const handleCloseFullscreen = useCallback(() => setIsFullscreen(false), []);

  // ─── Render ───────────────────────────────────────────────────────────────────

  return (
    <div className="root-app">
      <div className="app-container">
        <Sidebar activeTab={activeTab} onSelectTab={handleSelectTab} />

        <main className="main-content">
          <AppHeader
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            loading={loading}
            currentFolderPath={currentFolderPath}
            onSelectFolder={handleSelectFolder}
          />

          <div className="song-list-container">
            {activeTab === "artists" && !selectedArtist ? (
              <ArtistsGrid
                artistGroups={artistGroups}
                artistSortOrder={artistSortOrder}
                onToggleSortOrder={handleToggleArtistSortOrder}
                onSelectArtist={setSelectedArtist}
              />
            ) : (
              <SongTable
                songs={sortedSongs}
                currentSong={currentSong}
                selectedArtist={selectedArtist}
                sortField={sortField}
                sortDirection={sortDirection}
                loading={loading}
                currentFolderPath={currentFolderPath}
                onBackToArtists={handleBackToArtists}
                onSort={handleSort}
                onPlaySong={handlePlaySongFromList}
                onShufflePlay={startShufflePlay}
                onSelectFolder={handleSelectFolder}
              />
            )}
          </div>
        </main>
      </div>

      <PlayerBar
        currentSong={currentSong}
        currentCoverSrc={currentCoverSrc}
        currentTime={currentTime}
        isPlaying={isPlaying}
        isShuffle={isShuffle}
        repeatMode={repeatMode}
        isNormalizeVolume={isNormalizeVolume}
        isMuted={isMuted}
        volume={volume}
        showLyrics={showLyrics}
        onToggleShuffle={toggleShuffle}
        onPrev={handlePrevSong}
        onTogglePlay={togglePlayPause}
        onNext={handleNextSong}
        onToggleRepeat={toggleRepeatMode}
        onSeekDragStart={handleSeekDragStart}
        onSeekScrub={setCurrentTime}
        onSeekCommit={handleSeekCommit}
        onToggleNormalize={toggleNormalizeVolume}
        onToggleMute={toggleMute}
        onVolumeChange={handleVolumeChange}
        onVolumeWheel={handleVolumeWheel}
        onOpenLyrics={handleOpenLyrics}
        onOpenFullscreen={handleOpenFullscreen}
      />

      {isFullscreen && (
        <FullscreenPlayer
          currentSong={currentSong}
          currentCoverSrc={currentCoverSrc}
          currentTime={currentTime}
          isPlaying={isPlaying}
          isShuffle={isShuffle}
          repeatMode={repeatMode}
          isNormalizeVolume={isNormalizeVolume}
          isMuted={isMuted}
          volume={volume}
          showLyrics={showLyrics}
          lyrics={lyrics}
          lyricsLoading={lyricsLoading}
          activeLineIndex={activeLineIndex}
          onToggleLyrics={handleToggleLyricsVisibility}
          onClose={handleCloseFullscreen}
          onToggleShuffle={toggleShuffle}
          onPrev={handlePrevSong}
          onTogglePlay={togglePlayPause}
          onNext={handleNextSong}
          onToggleRepeat={toggleRepeatMode}
          onSeekDragStart={handleSeekDragStart}
          onSeekScrub={setCurrentTime}
          onSeekCommit={handleSeekCommit}
          onToggleNormalize={toggleNormalizeVolume}
          onToggleMute={toggleMute}
          onVolumeChange={handleVolumeChange}
          onVolumeWheel={handleVolumeWheel}
        />
      )}
    </div>
  );
}

export default App;
