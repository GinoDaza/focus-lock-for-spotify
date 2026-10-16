import type { ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

export function Page({
  title,
  description,
  children,
}: {
  title: string;
  description?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="page">
      <header className="page-header">
        <h1>{title}</h1>
        {description && <p className="secondary-text">{description}</p>}
      </header>
      {children}
    </div>
  );
}

export function Section({
  title,
  action,
  children,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="section">
      <div className="section-header">
        <h2>{title}</h2>
        {action}
      </div>
      <div className="cards">{children}</div>
    </section>
  );
}

/** A settings-style row: icon, header and description, with controls on the right. */
export function Card({
  icon,
  header,
  description,
  children,
  stacked,
}: {
  icon?: IconName;
  header?: ReactNode;
  description?: ReactNode;
  children?: ReactNode;
  /** Put the controls below the text instead of beside it. */
  stacked?: boolean;
}) {
  return (
    <div className={stacked ? "card stacked" : "card"}>
      {(icon || header) && (
        <div className="card-main">
          {icon && <Icon name={icon} className="card-icon" />}
          <div className="card-text">
            {header && <div className="card-header">{header}</div>}
            {description && <div className="caption secondary-text">{description}</div>}
          </div>
        </div>
      )}
      {children && <div className="card-controls">{children}</div>}
    </div>
  );
}

type Severity = "info" | "success" | "caution" | "critical";

const SEVERITY_ICON: Record<Severity, IconName> = {
  info: "info",
  success: "success",
  caution: "warning",
  critical: "error",
};

export function InfoBar({
  severity,
  title,
  icon,
  children,
  action,
}: {
  severity: Severity;
  title: string;
  icon?: IconName;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className={`infobar ${severity}`} role={severity === "critical" ? "alert" : "status"}>
      <Icon name={icon ?? SEVERITY_ICON[severity]} className="infobar-icon" />
      <div className="infobar-text">
        <strong>{title}</strong> {children}
      </div>
      {action}
    </div>
  );
}
