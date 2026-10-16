import type { DurationParts } from "../format";

const FIELDS = [
  ["days", "d", "Days", 30],
  ["hours", "h", "Hours", 23],
  ["minutes", "m", "Minutes", 59],
] as const;

export function DurationInput({
  value,
  onChange,
}: {
  value: DurationParts;
  onChange: (value: DurationParts) => void;
}) {
  return (
    <div className="duration">
      {FIELDS.map(([key, unit, label, max]) => (
        <label key={key} className="numberbox">
          <input
            type="number"
            min={0}
            max={max}
            aria-label={label}
            value={value[key]}
            onFocus={(e) => e.target.select()}
            onChange={(e) =>
              onChange({ ...value, [key]: Math.max(0, Math.floor(Number(e.target.value) || 0)) })
            }
          />
          <span className="unit">{unit}</span>
        </label>
      ))}
    </div>
  );
}
