import { useCallback, useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { ConfirmLock } from "../components/ConfirmLock";
import { DurationInput } from "../components/DurationInput";
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
  const [parts, setParts] = useState<DurationParts>({ days: 0, hours: 0, minutes: 3 });
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
    <section className="stack">
      {loadError && <p className="error-text">{loadError}</p>}

      <div className="card stack">
        <h2>Saved presets</h2>
        {presets?.length === 0 && <p className="muted">No presets yet.</p>}
        <ul className="presets">
          {presets?.map((preset) => (
            <li key={preset.name}>
              <div>
                <strong>{preset.name}</strong>
                <span className="muted">
                  {formatDuration(preset.duration)} · {preset.mode}
                </span>
              </div>
              <div className="row">
                <button
                  className="secondary"
                  disabled={active || action.busy}
                  title={active ? "A session is already running" : undefined}
                  onClick={() =>
                    preset.mode === "locked" ? setConfirming(preset) : void start(preset)
                  }
                >
                  Start
                </button>
                <button
                  className="link"
                  disabled={action.busy}
                  onClick={() =>
                    void action.run(() => api.deletePreset(preset.name)).then(() => load())
                  }
                >
                  Delete
                </button>
              </div>
            </li>
          ))}
        </ul>
      </div>

      <div className="card stack">
        <h2>New preset</h2>
        <label className="field">
          <span>Name</span>
          <input value={name} placeholder="Study" onChange={(e) => setName(e.target.value)} />
        </label>
        <DurationInput value={parts} onChange={setParts} />
        <ModeToggle value={mode} onChange={setMode} />
        <button
          className="primary"
          disabled={invalid !== null || action.busy}
          onClick={() => void save()}
        >
          Save preset
        </button>
        {action.error && !confirming && <p className="error-text">{action.error}</p>}
      </div>

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
    </section>
  );
}
