import { useState } from "react";
import { useStatus } from "./hooks";
import { PlaylistsView } from "./views/Playlists";
import { PresetsView } from "./views/Presets";
import { SessionView } from "./views/Session";

const TABS = [
  ["session", "Session"],
  ["playlists", "Playlists"],
  ["presets", "Presets"],
] as const;

type Tab = (typeof TABS)[number][0];

export function App() {
  const live = useStatus();
  const [tab, setTab] = useState<Tab>("session");
  const session = live.status?.session ?? null;

  return (
    <div className="app">
      <header className="header">
        <h1>Tunelock</h1>
        <span className={`pill ${session ? session.mode : "idle"}`}>
          {session ? (session.mode === "locked" ? "Locked" : "Active") : "Idle"}
        </span>
      </header>

      <nav className="tabs" role="tablist">
        {TABS.map(([id, label]) => (
          <button
            key={id}
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "tab selected" : "tab"}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </nav>

      <main className="content">
        {live.error && (
          <div className="banner error" role="alert">
            <strong>Can't reach the Tunelock service.</strong>
            <span>{live.error}</span>
            <button className="link" onClick={() => void live.refresh()}>
              Retry
            </button>
          </div>
        )}
        {live.status === null ? (
          !live.error && <p className="muted">Connecting…</p>
        ) : tab === "session" ? (
          <SessionView session={session} remaining={live.remaining} onChange={live.refresh} />
        ) : tab === "playlists" ? (
          <PlaylistsView session={session} />
        ) : (
          <PresetsView active={session !== null} onChange={live.refresh} />
        )}
      </main>

      <footer className="footer muted">
        {live.status ? `Service ${live.status.service_version}` : " "} · Closing this window does not
        end a session.
      </footer>
    </div>
  );
}
