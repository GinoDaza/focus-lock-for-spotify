import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { ConfirmLock } from "../components/ConfirmLock";
import { DurationInput } from "../components/DurationInput";
import { Icon } from "../components/Icon";
import { Card, InfoBar, Page, Section } from "../components/Layout";
import { ModeToggle } from "../components/ModeToggle";
import { type DurationParts, durationError, formatDuration, toSeconds } from "../format";
import { useAction } from "../hooks";
import type { LockMode, Preset } from "../protocol";

export function PresetsView({
  active,
  onChange,
}: {
  active: boolean;
  onChange: () => Promise<void>;
}) {
  const [presets, setPresets] = useState<Preset[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [parts, setParts] = useState<DurationParts>({ days: 0, hours: 0, minutes: 15 });
  const [mode, setMode] = useState<LockMode>("flexible");
  const [confirming, setConfirming] = useState<Preset | null>(null);
  const action = useAction();

  const load = useCallback(async () => {
    try {
      setPresets(await api.presets());
      setLoadError(null);
    } catch (e) {
      setLoadError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const seconds = toSeconds(parts);
  const invalid = name.trim() === "" ? "Give it a name." : durationError(seconds);
  const replaces = presets?.some((p) => p.name === name.trim()) ?? false;

  const save = async () => {
    const preset = { name: name.trim(), mode, duration: seconds };
    if (await action.run(() => api.savePreset(preset))) {
      setName("");
      await load();
    }
  };

  const start = async (preset: Preset) => {
    if (await action.run(() => api.startPreset(preset.name))) {
      setConfirming(null);
      await onChange();
    }
  };

  return (
    <Page title="Presets" description="Session lengths you use often, one click away.">
      {loadError && <InfoBar severity="critical" title="Couldn't load presets.">{loadError}</InfoBar>}

      <Section title="Saved">
        {presets?.length === 0 && (
          <div className="card empty secondary-text">No presets yet. Create one below.</div>
        )}
        {presets?.map((preset) => (
          <Card
            key={preset.name}
            icon={preset.mode === "locked" ? "lock" : "clock"}
            header={preset.name}
            description={`${formatDuration(preset.duration)} · ${preset.mode === "locked" ? "Locked" : "Flexible"}`}
          >
            <button
              disabled={active || action.busy}
              title={active ? "A session is already running" : undefined}
              onClick={() => (preset.mode === "locked" ? setConfirming(preset) : void start(preset))}
            >
              Start
            </button>
            <button
              className="subtle icon-button"
              title="Delete"
              aria-label={`Delete ${preset.name}`}
              disabled={action.busy}
              onClick={() => void action.run(() => api.deletePreset(preset.name)).then(() => load())}
            >
              <Icon name="delete" />
            </button>
          </Card>
        ))}
      </Section>

      <Section title="New preset">
        <Card icon="presets" header="Name">
          <input
            className="name-input"
            value={name}
            placeholder="Study"
            onChange={(e) => setName(e.target.value)}
          />
        </Card>
        <Card icon="stopwatch" header="Duration" description="Up to 30 days.">
          <DurationInput value={parts} onChange={setParts} />
        </Card>
        <Card icon="lock" header="Mode" stacked>
          <ModeToggle value={mode} onChange={setMode} />
        </Card>
        <div className="actions">
          {replaces && <span className="caption secondary-text">Replaces the preset with this name.</span>}
          {action.error && !confirming && <span className="error-text">{action.error}</span>}
          <button
            className="accent"
            disabled={invalid !== null || action.busy}
            onClick={() => void save()}
          >
            Save preset
          </button>
        </div>
      </Section>

      {confirming && (
        <ConfirmLock
          seconds={confirming.duration}
          busy={action.busy}
          error={action.error}
          onClose={() => {
            setConfirming(null);
            action.clearError();
          }}
          onConfirm={() => void start(confirming)}
        />
      )}
    </Page>
  );
}
