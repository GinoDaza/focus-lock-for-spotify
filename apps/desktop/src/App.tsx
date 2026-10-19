import { useEffect, useRef, useState } from "react";
import { Icon, type IconName } from "./components/Icon";
import { InfoBar } from "./components/Layout";
import { formatCountdown } from "./format";
import { useSetup, useStatus } from "./hooks";
import { setupReady } from "./protocol";
import { PlaylistsView } from "./views/Playlists";
import { PresetsView } from "./views/Presets";
import { SessionView } from "./views/Session";
import { SetupView } from "./views/Setup";

export type Tab = "session" | "playlists" | "presets" | "setup";

const NAV: [Tab, string, IconName][] = [
  ["session", "Session", "clock"],
  ["playlists", "Playlists", "music"],
  ["presets", "Presets", "presets"],
];

export function App() {
  const live = useStatus();
  const [tab, setTab] = useState<Tab>("session");
  const { setup, setSetup } = useSetup();
  const session = live.status?.session ?? null;
  const needsSetup = setup !== null && !setupReady(setup);

  // Open on Setup the first time it's needed, e.g. right after installing.
  const sentToSetup = useRef(false);
  useEffect(() => {
    if (needsSetup && !sentToSetup.current) {
      sentToSetup.current = true;
      setTab("setup");
    }
  }, [needsSetup]);

  const navItem = (id: Tab, label: string, icon: IconName, attention = false) => (
    <button
      className={tab === id ? "nav-item selected" : "nav-item"}
      aria-current={tab === id ? "page" : undefined}
      onClick={() => setTab(id)}
    >
      <Icon name={icon} />
      {label}
      {attention && <span className="dot" aria-label="needs attention" />}
    </button>
  );

  return (
    <div className="app">
      <nav className="nav" aria-label="Main">
        <div className="nav-title">
          <img src="/icon.png" alt="" />
          <span>Focus Lock</span>
        </div>
        <ul>
          {NAV.map(([id, label, icon]) => (
            <li key={id}>{navItem(id, label, icon)}</li>
          ))}
        </ul>
        <div className="nav-bottom">{navItem("setup", "Setup", "settings", needsSetup)}</div>
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
            title="Can't reach the Focus Lock service."
            action={<button onClick={() => void live.refresh()}>Retry</button>}
          >
            Make sure it's installed and running. If it won't start, check whether another program is using port 39817; the reason is in the service log. <span className="tertiary-text">({live.error})</span>
          </InfoBar>
        )}
        {tab === "setup" ? (
          <SetupView
            setup={setup}
            locked={session?.mode === "locked"}
            onChange={setSetup}
          />
        ) : live.status === null ? null : tab === "session" ? (
          <SessionView
            session={session}
            remaining={live.remaining}
            onChange={live.refresh}
            onNavigate={setTab}
            needsSetup={needsSetup}
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
