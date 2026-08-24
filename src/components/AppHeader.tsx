import { memo, startTransition } from "react";

interface AppHeaderProps {
  searchQuery: string;
  onSearchChange: (value: string) => void;
  loading: boolean;
  currentFolderPath: string | null;
  onSelectFolder: () => void;
}

export const AppHeader = memo(function AppHeader({
  searchQuery, onSearchChange, loading, currentFolderPath, onSelectFolder,
}: AppHeaderProps) {
  return (
    <header className="header">
      <div className="search-box">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="#9ca3af"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z" /></svg>
        <input
          type="text"
          placeholder="Buscar canciones, artistas..."
          value={searchQuery}
          onChange={e => startTransition(() => onSearchChange(e.target.value))}
        />
        {searchQuery && (
          <button
            style={{ background: "none", border: "none", color: "#9ca3af", cursor: "pointer", padding: "0 4px", fontSize: "16px", lineHeight: 1 }}
            onClick={() => startTransition(() => onSearchChange(""))}
            title="Borrar búsqueda"
          >✕</button>
        )}
      </div>

      <button className="btn-scan" onClick={onSelectFolder} disabled={loading}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="white"><path d="M20 6h-8l-2-2H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16v10z" /></svg>
        <span>{loading ? "Escaneando..." : currentFolderPath ? "Cambiar carpeta..." : "Abrir carpeta de música"}</span>
      </button>
    </header>
  );
});
