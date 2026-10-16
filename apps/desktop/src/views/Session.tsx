import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { ConfirmLock } from "../components/ConfirmLock";
import { DurationInput } from "../components/DurationInput";
import { ModeToggle } from "../components/ModeToggle";
import { PlaylistList } from "../components/PlaylistList";
import {
  type DurationParts,
  durationError,
  endsAt,
  formatCountdown,
  formatDuration,
  toSeconds,
} from "../format";
import { useAction } from "../hooks";
import type { Allowlist, LockMode, Preset, SessionInfo } from "../protocol";

type Refresh = () => Promise<void>;

export function SessionView({
  session,
  remaining,
  onChange,
}: {
  session: SessionInfo | null;
  remaining: number;
  onChange: Refresh;
}) {
  return session ? (
    <ActiveSession session={session} remaining={remaining} onChange={onChange} />
  ) : (
    <StartSession onChange={onChange} />
  );
}

const EXTEND = [
  ["+15m", 15 * 60],
  ["+1h", 3_600],
  ["+1d", 86_400],
] as const;

function ActiveSession({
  session,
  remaining,
  onChange,
}: {
  session: SessionInfo;
  remaining: number;
  onChange: Refresh;
}) {
  const action = useAction();
  const [confirming, setConfirming] = useState(false);
  const locked = session.mode === "locked";
  const progress = session.duration > 0 ? 1 - remaining / session.duration : 1;

  const act = async (run: () => Promise<void>) => {
    if (await action.run(run)) await onChange();
  };

  return (
    <section className="stack">
      <div className="card hero">
        <span className="muted">{locked ? "Locked session" : "Flexible session"}</span>
        <div className="countdown" aria-live="off">
          {formatCountdown(remaining)}
        </div>
        <div className="progress" aria-hidden="true">
          <div style={{ width: `${Math.min(100, Math.max(0, progress * 100))}%` }} />
        </div>
        <span className="muted">
          Ends {endsAt(remaining, new Date())} · {formatDuration(session.duration)} in total
        </span>
      </div>

      <div className="card stack">
        <h2>Add time</h2>
        <div className="row">
          {EXTEND.map(([label, seconds]) => (
            <button
              key={label}
              className="secondary"
              disabled={action.busy}
              onClick={() => void act(() => api.extend(seconds))}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {locked ? (
        <p className="muted">This session is locked. It can't be ended early, only extended.</p>
      ) : (
        <div className="row">
          <button className="danger" disabled={action.busy} onClick={() => setConfirming(true)}>
            Lock it
          </button>
          <button
            className="secondary"
            disabled={action.busy}
            onClick={() => void act(api.cancel)}
          >
            End session
          </button>
        </div>
      )}
      {action.error && !confirming && <p className="error-text">{action.error}</p>}

      <div className="card stack">
        <h2>Allowed playlists</h2>
        <PlaylistList ids={session.allowlist} />
      </div>

      {confirming && (
        <ConfirmLock
          seconds={remaining}
          busy={action.busy}
          error={action.error}
          onClose={() => {
            setConfirming(false);
            action.clearError();
          }}
          onConfirm={() =>
            void action.run(api.harden).then(async (ok) => {
              if (!ok) return;
              setConfirming(false);
              await onChange();
            })
          }
        />
      )}
    </section>
  );
}

interface Pending {
  seconds: number;
  start: () => Promise<void>;
}

function StartSession({ onChange }: { onChange: Refresh }) {
  const [allowlist, setAllowlist] = useState<Allowlist | null>(null);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [parts, setParts] = useState<DurationParts>({ days: 0, hours: 0, minutes: 3 });
  const [mode, setMode] = useState<LockMode>("flexible");
  const [pending, setPending] = useState<Pending | null>(null);
  const action = useAction();

  useEffect(() => {
    Promise.all([api.allowlist(), api.presets()])
      .then(([ids, saved]) => {
        setAllowlist(ids);
        setPresets(saved);
      })
      .catch((e: unknown) => setLoadError(errorMessage(e)));
  }, []);

  const seconds = toSeconds(parts);
  const invalid = durationError(seconds);
  const noPlaylists = allowlist !== null && allowlist.length === 0;

  const begin = async (lockMode: LockMode, duration: number, start: () => Promise<void>) => {
    if (lockMode === "locked") {
      setPending({ seconds: duration, start });
    } else if (await action.run(start)) {
      await onChange();
    }
  };

  return (
    <section className="stack">
      {loadError && <p className="error-text">{loadError}</p>}
      {noPlaylists && (
        <div className="banner">Add at least one playlist in the Playlists tab first.</div>
      )}

      {presets.length > 0 && (
        <div className="card stack">
          <h2>Presets</h2>
          <div className="row wrap">
            {presets.map((preset) => (
              <button
                key={preset.name}
                className="chip"
                disabled={action.busy || noPlaylists}
                onClick={() =>
                  void begin(preset.mode, preset.duration, () => api.startPreset(preset.name))
                }
              >
                <strong>{preset.name}</strong>
                <span>
                  {formatDuration(preset.duration)}
                  {preset.mode === "locked" ? " · locked" : ""}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}

      <div className="card stack">
        <h2>New session</h2>
        <DurationInput value={parts} onChange={setParts} />
        <ModeToggle value={mode} onChange={setMode} />
        {invalid && seconds > 0 && <p className="error-text">{invalid}</p>}
        <button
          className="primary"
          disabled={action.busy || invalid !== null || allowlist === null || noPlaylists}
          onClick={() => {
            if (allowlist) void begin(mode, seconds, () => api.start(mode, seconds, allowlist));
          }}
        >
          Start {formatDuration(seconds)}
        </button>
        {action.error && !pending && <p className="error-text">{action.error}</p>}
      </div>

      {allowlist && allowlist.length > 0 && (
        <div className="card stack">
          <h2>Allowed playlists</h2>
          <PlaylistList ids={allowlist} />
        </div>
      )}

      {pending && (
        <ConfirmLock
          seconds={pending.seconds}
          busy={action.busy}
          error={action.error}
          onClose={() => {
            setPending(null);
            action.clearError();
          }}
          onConfirm={() =>
            void action.run(pending.start).then(async (ok) => {
              if (!ok) return;
              setPending(null);
              await onChange();
            })
          }
        />
      )}
    </section>
  );
}
