import { ReactNode } from "react";

export function PrimaryButton({
  children,
  onClick,
  disabled,
  type = "button",
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  type?: "button" | "submit";
}) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className="inline-flex items-center gap-2 rounded-lg bg-brand-500 px-5 py-2.5 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-brand-600 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-500 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {children}
    </button>
  );
}

export function SecondaryButton({
  children,
  onClick,
  disabled,
}: {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className="inline-flex items-center gap-1.5 rounded-md bg-stone-100 px-3 py-1.5 text-sm font-medium text-stone-700 transition-colors hover:bg-stone-200 disabled:cursor-not-allowed disabled:opacity-50"
    >
      {children}
    </button>
  );
}

export function Card({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`rounded-2xl border border-stone-200 bg-white p-6 shadow-sm sm:p-8 ${className}`}
    >
      {children}
    </div>
  );
}

export function Callout({
  variant = "info",
  title,
  children,
}: {
  variant?: "info" | "warn" | "error" | "success";
  title?: string;
  children: ReactNode;
}) {
  const styles = {
    info: "border-stone-200 bg-stone-50 text-stone-700",
    warn: "border-amber-200 bg-amber-50 text-amber-900",
    error: "border-red-200 bg-red-50/60 text-red-800",
    success: "border-brand-200 bg-brand-50 text-brand-700",
  }[variant];
  return (
    <div className={`rounded-lg border px-4 py-3 text-sm/6 ${styles}`}>
      {title && <p className="font-medium">{title}</p>}
      <div className={title ? "mt-1" : ""}>{children}</div>
    </div>
  );
}

export function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <span className="text-xs font-semibold uppercase tracking-wider text-brand-500">
      {children}
    </span>
  );
}

export function Heading({ children }: { children: ReactNode }) {
  return (
    <h1 className="mt-2 text-balance font-serif text-3xl font-medium tracking-tight text-stone-900 sm:text-4xl">
      {children}
    </h1>
  );
}

export function Subhead({ children }: { children: ReactNode }) {
  return (
    <p className="mt-3 max-w-[60ch] text-pretty text-base/7 text-stone-500">
      {children}
    </p>
  );
}

export function StatusDot({
  variant,
}: {
  variant: "idle" | "active" | "ok" | "error";
}) {
  const cls = {
    idle: "bg-stone-300",
    active: "bg-amber-500 animate-pulse-dot",
    ok: "bg-brand-500",
    error: "bg-red-500",
  }[variant];
  return <span className={`inline-block size-2 shrink-0 rounded-full ${cls}`} />;
}

export function StepNumber({
  n,
  active,
}: {
  n: number;
  active?: boolean;
}) {
  return (
    <span
      className={`flex size-7 shrink-0 items-center justify-center rounded-full font-mono text-xs font-semibold ${
        active ? "bg-brand-500 text-white" : "bg-stone-200 text-stone-500"
      }`}
    >
      {n}
    </span>
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="mb-6">
      <label className="block text-sm/6 font-medium text-stone-700">
        {label}
      </label>
      <div className="mt-1.5">{children}</div>
      {hint && <p className="mt-1.5 text-xs text-stone-400">{hint}</p>}
    </div>
  );
}

export function Pill({
  children,
  variant = "neutral",
}: {
  children: ReactNode;
  variant?: "neutral" | "brand" | "amber" | "red" | "indigo";
}) {
  const cls = {
    neutral: "bg-stone-100 text-stone-700",
    brand: "bg-brand-50 text-brand-500",
    amber: "bg-amber-100 text-amber-800",
    red: "bg-red-50 text-red-700",
    indigo: "bg-indigo-50 text-indigo-600",
  }[variant];
  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold ${cls}`}
    >
      {children}
    </span>
  );
}
