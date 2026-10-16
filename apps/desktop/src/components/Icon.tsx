// Glyphs from Segoe Fluent Icons (Windows 11), with Segoe MDL2 Assets
// (Windows 10) as fallback: both ship with Windows and share code points.

const GLYPHS = {
  add: "",
  cancel: "",
  clock: "",
  delete: "",
  error: "",
  info: "",
  lock: "",
  music: "",
  play: "",
  presets: "",
  stopwatch: "",
  success: "",
  warning: "",
} as const;

export type IconName = keyof typeof GLYPHS;

export function Icon({ name, className }: { name: IconName; className?: string }) {
  return (
    <span className={className ? `icon ${className}` : "icon"} aria-hidden="true">
      {GLYPHS[name]}
    </span>
  );
}
