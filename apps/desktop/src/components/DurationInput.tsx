import type { DurationParts } from "../format";

const FIELDS = [
  ["days", "Days", 30],
  ["hours", "Hours", 23],
  ["minutes", "Minutes", 59],
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
      {FIELDS.map(([key, label, max]) => (
        <label key={key}>
          <span>{label}</span>
          <input
            type="number"
            min={0}
            max={max}
            value={value[key]}
            onChange={(e) =>
              onChange({ ...value, [key]: Math.max(0, Math.floor(Number(e.target.value) || 0)) })
            }
          />
        </label>
      ))}
    </div>
  );
}
