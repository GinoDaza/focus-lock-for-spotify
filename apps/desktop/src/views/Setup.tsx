import { useState } from "react";
import { api, errorMessage } from "../api";
import { Icon, type IconName } from "../components/Icon";
import { Card, InfoBar, Page, Section } from "../components/Layout";
import { type SetupStatus, setupReady } from "../protocol";

type StepState = "ready" | "todo" | "blocked";

function StepBadge({ state }: { state: StepState }) {
  const [icon, label]: [IconName, string] =
    state === "ready" ? ["success", "Done"] : state === "todo" ? ["info", "To do"] : ["error", "Action needed"];
  return (
    <span className={`badge ${state}`}>
      <Icon name={icon} />
      {label}
    </span>
  );
}

export function SetupView({
  setup,
  locked,
  onChange,
}: {
  setup: SetupStatus | null;
  locked: boolean;
  onChange: (status: SetupStatus) => void;
}) {
  const [step, setStep] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [finished, setFinished] = useState(false);

  if (!setup) return null;

  const ready = setupReady(setup);
  const spotifyOk = setup.spotify === "ready";
  const extensionLabel =
    setup.extension === "ready"
      ? "Installed and applied."
      : setup.extension === "outdated"
        ? "A newer version comes with this app."
        : "Not in Spotify yet.";

  const run = async () => {
    setError(null);
    setFinished(false);
    setStep("Starting…");
    try {
      onChange(await api.runSetup(setStep));
      setFinished(true);
    } catch (e) {
      setError(errorMessage(e));
      onChange(await api.setupStatus().catch(() => setup));
    } finally {
      setStep(null);
    }
  };

  return (
    <Page
      title="Setup"
      description="Focus Lock works inside the Spotify desktop app through Spicetify, a tool that lets Spotify load extensions."
    >
      {locked && (
        <InfoBar severity="info" icon="lock" title="Setup is paused during a locked session.">
          Focus Lock keeps Spotify set up on its own until it ends.
        </InfoBar>
      )}
      {finished && ready && (
        <InfoBar severity="success" title="All set.">
          Spotify restarted with Focus Lock. You can start a session.
        </InfoBar>
      )}
      {error && (
        <InfoBar severity="critical" title="Setup didn't finish.">
          {error}
        </InfoBar>
      )}

      <Section title="Steps">
        <Card
          icon="music"
          header="Spotify desktop app"
          description={
            setup.spotify === "ready"
              ? "Installed from spotify.com."
              : setup.spotify === "store"
                ? "The Microsoft Store version can't load extensions. Install Spotify from spotify.com instead."
                : "Install Spotify for Windows from spotify.com."
          }
        >
          {!spotifyOk && (
            <button onClick={() => void api.openSpotifyDownload()}>
              <Icon name="open" />
              Get Spotify
            </button>
          )}
          <StepBadge state={spotifyOk ? "ready" : "blocked"} />
        </Card>
        <Card
          icon="settings"
          header="Spicetify"
          description={
            setup.spicetify
              ? "Installed."
              : "Will be downloaded from its official GitHub releases (spicetify/cli)."
          }
        >
          <StepBadge state={setup.spicetify ? "ready" : "todo"} />
        </Card>
        <Card icon="lock" header="Focus Lock in Spotify" description={extensionLabel}>
          <StepBadge state={setup.extension === "ready" ? "ready" : "todo"} />
        </Card>
      </Section>

      {!ready && (
        <div className="actions">
          {step && (
            <span className="secondary-text progress-label">
              <span className="spinner" aria-hidden="true" />
              {step}
            </span>
          )}
          {!step && spotifyOk && (
            <span className="caption secondary-text">Spotify will restart.</span>
          )}
          <button
            className="accent"
            disabled={!spotifyOk || locked || step !== null}
            onClick={() => void run()}
          >
            {setup.extension === "outdated" ? "Update" : "Set up"}
          </button>
        </div>
      )}
    </Page>
  );
}
