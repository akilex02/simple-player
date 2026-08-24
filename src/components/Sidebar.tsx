import { memo, startTransition } from "react";
import type { ActiveTab } from "../types";

interface SidebarProps {
  activeTab: ActiveTab;
  onSelectTab: (tab: ActiveTab) => void;
}

export const Sidebar = memo(function Sidebar({ activeTab, onSelectTab }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <img src="/app-icon.png" alt="Simple Player" className="brand-logo-img" />
        <span className="brand-name">Simple Player</span>
      </div>

      <div className="nav-section">
        <span className="nav-title">Navegación</span>
        <div
          className={`nav-item ${activeTab === "all" ? "active" : ""}`}
          onClick={() => startTransition(() => onSelectTab("all"))}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3L1 9l11 6 9-4.91V17h2V9L12 3zM3.27 9L12 4.24 20.73 9 12 13.76 3.27 9z" /></svg>
          <span>Toda la Música</span>
        </div>

        <div
          className={`nav-item ${activeTab === "artists" ? "active" : ""}`}
          onClick={() => startTransition(() => onSelectTab("artists"))}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z" /></svg>
          <span>Artistas</span>
        </div>
      </div>
    </aside>
  );
});
