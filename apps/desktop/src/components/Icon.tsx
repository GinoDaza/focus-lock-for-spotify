// Glyphs from Segoe Fluent Icons (Windows 11), with Segoe MDL2 Assets
// (Windows 10) as fallback: both ship with Windows and share code points.

const GLYPHS = {
  add: "\uE710",
  cancel: "\uE711",
  chevronDown: "\uE70D",
  chevronUp: "\uE70E",
  clock: "\uE823",
  delete: "\uE74D",
  error: "\uE783",
  info: "\uE946",
  lock: "\uE72E",
  music: "\uE8D6",
  open: "\uE8A7",
  play: "\uE768",
  presets: "\uE8A4",
  settings: "\uE713",
  stopwatch: "\uE916",
  success: "\uE930",
  warning: "\uE7BA",
} as const;

export type IconName = keyof typeof GLYPHS;

export function Icon({ name, className }: { name: IconName; className?: string }) {
  return (
    <span className={className ? `icon ${className}` : "icon"} aria-hidden="true">
      {GLYPHS[name]}
    </span>
  );
}
