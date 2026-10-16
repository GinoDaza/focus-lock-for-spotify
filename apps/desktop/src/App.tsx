import { useState } from "react";
import { Icon, type IconName } from "./components/Icon";
import { InfoBar } from "./components/Layout";
import { formatCountdown } from "./format";
import { useStatus } from "./hooks";
import { PlaylistsView } from "./views/Playlists";
import { PresetsView } from "./views/Presets";
import { SessionView } from "./views/Session";

export type Tab = "session" | "playlists" | "presets";

const NAV: [Tab, string, IconName][] = [
  ["session", "Session", "clock"],
  ["playlists", "Playlists", "music"],
  ["presets", "Presets", "presets"],
];

export function App() {
  const live = useStatus();
  const [tab, setTab] = useState<Tab>("session");
  const session = live.status?.session ?? null;

  return (
    <div className="app">
      <nav className="nav" aria-label="Main">
        <div className="nav-title">
          <img src="/icon.png" alt="" />
          <span>Tunelock</span>
        </div>
        <ul>
          {NAV.map(([id, label, icon]) => (
            <li key={id}>
              <button
                className={tab === id ? "nav-item selected" : "nav-item"}
                aria-current={tab === id ? "page" : undefined}
                onClick={() => setTab(id)}
              >
                <Icon name={icon} />
                {label}
              </button>
            </li>
          ))}
        </ul>
        <div className="nav-status" aria-live="polite">
          {session ? (
            <>
              <Icon name={session.mode === "locked" ? "lock" : "clock"} />
              <span>
                {session.mode === "locked" ? "Locked" : "Active"} ·{" "}
                <span className="tabular">{formatCountdown(live.remaining)}</span>
              </span>
            </>
          ) : (
            <span className="tertiary-text">
              {live.error ? "Service unavailable" : live.status ? "No active session" : "Connecting…"}
            </span>
          )}
        </div>
      </nav>

      <main className="content">
        {live.error && (
          <InfoBar
            severity="critical"
            title="Can't reach the Tunelock service."
            action={<button onClick={() => void live.refresh()}>Retry</button>}
          >
            Make sure it's installed and running. <span className="tertiary-text">({live.error})</span>
          </InfoBar>
        )}
        {live.status === null ? null : tab === "session" ? (
          <SessionView
            session={session}
            remaining={live.remaining}
            onChange={live.refresh}
            onNavigate={setTab}
          />
        ) : tab === "playlists" ? (
          <PlaylistsView session={session} />
        ) : (
          <PresetsView active={session !== null} onChange={live.refresh} />
        )}
        {live.status && (
          <p className="footnote caption tertiary-text">
            Closing this window doesn't end a session. Service {live.status.service_version}.
          </p>
        )}
      </main>
    </div>
  );
}
