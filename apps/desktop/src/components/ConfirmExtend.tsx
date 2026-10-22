import { endsAt, formatDuration } from "../format";

/** Confirms adding time to a locked session, which can't be taken back. */
export function ConfirmExtend({
  seconds,
  remaining,
  busy,
  error,
  onConfirm,
  onClose,
}: {
  seconds: number;
  remaining: number;
  busy: boolean;
  error: string | null;
  onConfirm: () => void;
  onClose: () => void;
}) {
  return (
    <div className="smoke" onMouseDown={onClose}>
      <div
        className="dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="extend-title"
        aria-describedby="extend-body"
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") onClose();
        }}
      >
        <div className="dialog-body">
          <h2 id="extend-title">Add {formatDuration(seconds)}?</h2>
          <p id="extend-body">
            This session is locked, so the extra time can't be taken back. It will end at{" "}
            <strong>{endsAt(remaining + seconds, new Date())}</strong>.
          </p>
          {error && <p className="error-text">{error}</p>}
        </div>
        <div className="dialog-footer">
          <button className="accent" disabled={busy} onClick={onConfirm}>
            Add time
          </button>
          <button autoFocus onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
