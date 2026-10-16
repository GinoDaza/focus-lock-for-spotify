import type { LockMode } from "../protocol";

export function ModeToggle({
  value,
  onChange,
}: {
  value: LockMode;
  onChange: (mode: LockMode) => void;
}) {
  return (
    <div className="segmented" role="radiogroup" aria-label="Mode">
      <button role="radio" aria-checked={value === "flexible"} onClick={() => onChange("flexible")}>
        <strong>Flexible</strong>
        <span>You can end it early.</span>
      </button>
      <button role="radio" aria-checked={value === "locked"} onClick={() => onChange("locked")}>
        <strong>Locked</strong>
        <span>No way back until it ends.</span>
      </button>
    </div>
  );
}
