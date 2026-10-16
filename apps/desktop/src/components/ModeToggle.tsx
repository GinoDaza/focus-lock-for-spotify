import type { LockMode } from "../protocol";

const OPTIONS: [LockMode, string, string][] = [
  ["flexible", "Flexible", "You can end it early."],
  ["locked", "Locked", "Can't be ended until the time runs out."],
];

export function ModeToggle({
  value,
  onChange,
}: {
  value: LockMode;
  onChange: (mode: LockMode) => void;
}) {
  return (
    <div className="radios" role="radiogroup" aria-label="Mode">
      {OPTIONS.map(([mode, label, hint]) => (
        <label key={mode} className="radio">
          <input
            type="radio"
            name="mode"
            checked={value === mode}
            onChange={() => onChange(mode)}
          />
          <span>
            {label}
            <span className="caption secondary-text"> · {hint}</span>
          </span>
        </label>
      ))}
    </div>
  );
}
