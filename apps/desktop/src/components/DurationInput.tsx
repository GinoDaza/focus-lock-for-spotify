import { useEffect, useRef } from "react";
import type { DurationParts } from "../format";
import { Icon } from "./Icon";

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
        <NumberBox
          key={key}
          label={label}
          unit={unit}
          max={max}
          value={value[key]}
          onChange={(n) => onChange({ ...value, [key]: n })}
        />
      ))}
    </div>
  );
}

const REPEAT_DELAY_MS = 400;
const REPEAT_EVERY_MS = 60;

/** WinUI NumberBox with inline spin buttons: click or hold, wheel, ↑/↓. */
function NumberBox({
  label,
  unit,
  max,
  value,
  onChange,
}: {
  label: string;
  unit: string;
  max: number;
  value: number;
  onChange: (value: number) => void;
}) {
  const box = useRef<HTMLDivElement>(null);
  // Latest value for the timers and the native wheel listener.
  const current = useRef(value);
  current.current = value;
  const change = useRef(onChange);
  change.current = onChange;
  const timer = useRef<number | undefined>(undefined);

  const step = (delta: number) => {
    const next = Math.min(max, Math.max(0, current.current + delta));
    if (next !== current.current) {
      current.current = next;
      change.current(next);
    }
  };

  const stop = () => {
    clearTimeout(timer.current);
    clearInterval(timer.current);
    timer.current = undefined;
  };

  const press = (delta: number) => {
    step(delta);
    stop();
    timer.current = window.setTimeout(() => {
      timer.current = window.setInterval(() => step(delta), REPEAT_EVERY_MS);
    }, REPEAT_DELAY_MS);
  };

  useEffect(() => stop, []);

  // React's wheel handler is passive and can't stop the page from scrolling.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      step(e.deltaY < 0 ? 1 : -1);
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [max]);

  return (
    <div className="numberbox" ref={box}>
      <input
        type="number"
        min={0}
        max={max}
        aria-label={label}
        value={value}
        onFocus={(e) => e.target.select()}
        onChange={(e) =>
          onChange(Math.min(max, Math.max(0, Math.floor(Number(e.target.value) || 0))))
        }
      />
      <span className="unit">{unit}</span>
      <div className="spin">
        {([1, -1] as const).map((delta) => (
          <button
            key={delta}
            type="button"
            tabIndex={-1}
            aria-label={`${delta > 0 ? "Increase" : "Decrease"} ${label.toLowerCase()}`}
            disabled={delta > 0 ? value >= max : value <= 0}
            onPointerDown={(e) => {
              e.preventDefault();
              press(delta);
            }}
            onPointerUp={stop}
            onPointerLeave={stop}
          >
            <Icon name={delta > 0 ? "chevronUp" : "chevronDown"} />
          </button>
        ))}
      </div>
    </div>
  );
}
