import { useState } from "react";
import { endsAt, formatDuration } from "../format";

const WORD = "lock";

/** The strong confirmation required before any session becomes locked. */
export function ConfirmLock({
  seconds,
  busy,
  error,
  onConfirm,
  onClose,
}: {
  seconds: number;
  busy: boolean;
  error: string | null;
  onConfirm: () => void;
  onClose: () => void;
}) {
  const [typed, setTyped] = useState("");
  const ready = typed.trim().toLowerCase() === WORD;

  return (
    <div className="backdrop" onClick={onClose}>
      <div
        className="dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="confirm-title">Lock this session?</h2>
        <p>
          You won't be able to end it for <strong>{formatDuration(seconds)}</strong>, until{" "}
          <strong>{endsAt(seconds, new Date())}</strong>. Spotify will only play your allowed
          playlists until then, and you can only add time.
        </p>
        <label className="field">
          <span>
            Type <code>{WORD}</code> to confirm
          </span>
          <input
            autoFocus
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && ready && !busy) onConfirm();
              if (e.key === "Escape") onClose();
            }}
          />
        </label>
        {error && <p className="error-text">{error}</p>}
        <div className="row end">
          <button className="secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="danger" disabled={!ready || busy} onClick={onConfirm}>
            Lock
          </button>
        </div>
      </div>
    </div>
  );
}
