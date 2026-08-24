import { convertFileSrc } from "@tauri-apps/api/core";

// Convert absolute cover path to Tauri asset:// URL (WebKitGTK trusted origin)
export function getCoverUrl(coverPath: string | null): string | null {
  if (!coverPath) return null;
  // Already a data: URL - return as-is
  if (coverPath.startsWith('data:')) return coverPath;
  // Absolute file path -> convertFileSrc turns it into asset://localhost/...
  return convertFileSrc(coverPath);
}

export function shuffleArray<T>(array: T[]): T[] {
  const arr = [...array];
  for (let i = arr.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [arr[i], arr[j]] = [arr[j], arr[i]];
  }
  return arr;
}

export function formatTime(seconds: number): string {
  if (isNaN(seconds)) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
}
