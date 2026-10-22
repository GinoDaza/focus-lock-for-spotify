import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import type { Tab } from "../App";
import { ConfirmExtend } from "../components/ConfirmExtend";
import { ConfirmLock } from "../components/ConfirmLock";
import { DurationInput } from "../components/DurationInput";
import { Icon } from "../components/Icon";
import { Card, InfoBar, Page, Section } from "../components/Layout";
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
  onNavigate,
  needsSetup,
}: {
  session: SessionInfo | null;
  remaining: number;
  onChange: Refresh;
  onNavigate: (tab: Tab) => void;
  needsSetup: boolean;
}) {
  return session ? (
    <ActiveSession session={session} remaining={remaining} onChange={onChange} />
  ) : (
    <StartSession onChange={onChange} onNavigate={onNavigate} needsSetup={needsSetup} />
  );
}

const EXTEND = [
  ["+15 min", 15 * 60],
  ["+1 hour", 3_600],
  ["+1 day", 86_400],
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
  const [extending, setExtending] = useState<number | null>(null);
  const locked = session.mode === "locked";
  const progress = session.duration > 0 ? 1 - remaining / session.duration : 1;

  const act = async (run: () => Promise<void>) => {
    if (await action.run(run)) await onChange();
  };

  return (
    <Page title="Session">
      {locked && (
        <InfoBar severity="caution" icon="lock" title="Locked.">
          You can add time, but this session can't be ended early.
        </InfoBar>
      )}

      <div className="card hero">
        <span className="secondary-text">Time remaining</span>
        <div className="countdown">{formatCountdown(remaining)}</div>
        <div
          className="progress"
          role="progressbar"
          aria-label="Session progress"
          aria-valuenow={Math.round(progress * 100)}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div style={{ width: `${Math.min(100, Math.max(0, progress * 100))}%` }} />
        </div>
        <span className="caption secondary-text">
          Ends at {endsAt(remaining, new Date())} · {formatDuration(session.duration)}{" "}
          {locked ? "locked" : "flexible"} session
        </span>
      </div>

      <Section title="Adjust">
        <Card icon="stopwatch" header="Add time" description="Extends the current session.">
          {EXTEND.map(([label, seconds]) => (
            <button
              key={label}
              disabled={action.busy}
              onClick={() => {
                if (locked) setExtending(seconds);
                else void act(() => api.extend(seconds));
              }}
            >
              {label}
            </button>
          ))}
        </Card>
        {!locked && (
          <>
            <Card
              icon="lock"
              header="Lock session"
              description="Commit to the remaining time. This can't be undone."
            >
              <button disabled={action.busy} onClick={() => setConfirming(true)}>
                Lock…
              </button>
            </Card>
            <Card icon="cancel" header="End session" description="Spotify goes back to normal.">
              <button disabled={action.busy} onClick={() => void act(api.cancel)}>
                End now
              </button>
            </Card>
          </>
        )}
        {action.error && !confirming && extending === null && <p className="error-text">{action.error}</p>}
      </Section>

      <Section title="Allowed playlists">
        <PlaylistList ids={session.allowlist} />
      </Section>

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

      {extending !== null && (
        <ConfirmExtend
          seconds={extending}
          remaining={remaining}
          busy={action.busy}
          error={action.error}
          onClose={() => {
            setExtending(null);
            action.clearError();
          }}
          onConfirm={() =>
            void action.run(() => api.extend(extending)).then(async (ok) => {
              if (!ok) return;
              setExtending(null);
              await onChange();
            })
          }
        />
      )}
    </Page>
  );
}

interface Pending {
  seconds: number;
  start: () => Promise<void>;
}

function StartSession({
  onChange,
  onNavigate,
  needsSetup,
}: {
  onChange: Refresh;
  onNavigate: (tab: Tab) => void;
  needsSetup: boolean;
}) {
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
    <Page title="Session" description="Focus with only your allowed playlists in Spotify.">
      {loadError && <InfoBar severity="critical" title="Couldn't load your settings.">{loadError}</InfoBar>}
      {needsSetup && (
        <InfoBar
          severity="caution"
          title="Spotify isn't set up yet."
          action={<button onClick={() => onNavigate("setup")}>Set up</button>}
        >
          Sessions can't restrict Spotify until it is.
        </InfoBar>
      )}
      {noPlaylists && (
        <InfoBar
          severity="info"
          title="No playlists yet."
          action={<button onClick={() => onNavigate("playlists")}>Add playlists</button>}
        >
          Add at least one before starting a session.
        </InfoBar>
      )}

      <Section title="New session">
        <Card icon="stopwatch" header="Duration" description="Up to 30 days.">
          <DurationInput value={parts} onChange={setParts} />
        </Card>
        <Card icon="lock" header="Mode" stacked>
          <ModeToggle value={mode} onChange={setMode} />
        </Card>
        <div className="actions">
          {invalid && seconds > 0 && <span className="error-text">{invalid}</span>}
          {action.error && !pending && <span className="error-text">{action.error}</span>}
          <button
            className="accent"
            disabled={action.busy || invalid !== null || allowlist === null || noPlaylists}
            onClick={() => {
              if (allowlist) void begin(mode, seconds, () => api.start(mode, seconds, allowlist));
            }}
          >
            <Icon name="play" />
            Start {invalid ? "" : formatDuration(seconds)}
          </button>
        </div>
      </Section>

      {presets.length > 0 && (
        <Section title="Presets">
          {presets.map((preset) => (
            <Card
              key={preset.name}
              icon={preset.mode === "locked" ? "lock" : "clock"}
              header={preset.name}
              description={`${formatDuration(preset.duration)} · ${preset.mode === "locked" ? "Locked" : "Flexible"}`}
            >
              <button
                disabled={action.busy || noPlaylists}
                onClick={() =>
                  void begin(preset.mode, preset.duration, () => api.startPreset(preset.name))
                }
              >
                Start
              </button>
            </Card>
          ))}
        </Section>
      )}

      {allowlist && allowlist.length > 0 && (
        <Section
          title="Allowed playlists"
          action={
            <button className="hyperlink" onClick={() => onNavigate("playlists")}>
              Edit
            </button>
          }
        >
          <PlaylistList ids={allowlist} />
        </Section>
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
    </Page>
  );
}
