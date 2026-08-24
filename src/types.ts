export interface Song {
  path: string;
  title: string;
  artist: string;
  album: string;
  duration_secs: number;
  cover_art: string | null;
}

export interface PlaybackState {
  folder_path: string | null;
  queue_paths: string[];
  current_index: number | null;
  volume: number;
  is_shuffle: boolean;
  repeat_mode: string;
}

export type SortField = "title" | "artist" | "album" | "duration";
export type ActiveTab = "all" | "artists";
export type RepeatMode = "off" | "all" | "one";
