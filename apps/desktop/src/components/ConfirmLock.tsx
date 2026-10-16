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
    <div className="smoke" onMouseDown={onClose}>
      <div
        className="dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        aria-describedby="confirm-body"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="dialog-body">
          <h2 id="confirm-title">Lock this session?</h2>
          <p id="confirm-body">
            You won't be able to end it for <strong>{formatDuration(seconds)}</strong>, until{" "}
            <strong>{endsAt(seconds, new Date())}</strong>. Until then Spotify only plays your
            allowed playlists, and you can only add time.
          </p>
          <label className="field">
            <span>
              Type <strong>{WORD}</strong> to confirm
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
        </div>
        <div className="dialog-footer">
          <button className="accent" disabled={!ready || busy} onClick={onConfirm}>
            Lock session
          </button>
          <button onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );
}
