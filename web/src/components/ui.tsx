import type { ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { cn } from "../lib/utils";
import { prettyHotkey } from "../lib/hotkeys";

// Quad mark — "offset-tail tile" (2×2 rounded tiles; the offset bottom-right tile is the
// teal accent, forming a q). Uses our design tokens so it tracks the theme.
export function Logo({ size = 28 }: { size?: number }) {
  const tile = { fill: "oklch(var(--foreground) / 0.42)" };
  const accent = { fill: "oklch(var(--accent))" };
  return (
    <svg width={size} height={size} viewBox="0 0 100 100" className="shrink-0" aria-hidden="true">
      <rect x="22" y="22" width="24" height="24" rx="7" style={tile} />
      <rect x="54" y="22" width="24" height="24" rx="7" style={tile} />
      <rect x="22" y="54" width="24" height="24" rx="7" style={tile} />
      <rect x="58" y="58" width="24" height="24" rx="7" style={accent} />
    </svg>
  );
}

export function Loading() {
  return (
    <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
      <motion.div
        className="mr-3 h-4 w-4 rounded-full border-2 border-border border-t-accent"
        animate={{ rotate: 360 }}
        transition={{ repeat: Infinity, ease: "linear", duration: 0.8 }}
      />
      Connecting to Quad…
    </div>
  );
}

export function Stat({ label, value, warn }: { label: string; value: ReactNode; warn?: boolean }) {
  return (
    <div
      className={cn(
        "rounded-lg border px-3 py-2",
        warn ? "border-destructive/40 bg-destructive/10" : "border-border bg-card/60",
      )}
    >
      <div className={cn("text-base font-semibold", warn ? "text-destructive" : "text-foreground")}>{value}</div>
      <div className="text-[10px] uppercase tracking-wider text-muted-foreground">{label}</div>
    </div>
  );
}

export function Kbd({ spec, bound, registered }: { spec: string; bound: boolean; registered: boolean }) {
  if (!bound) {
    return <span className="rounded border border-border/60 px-2 py-0.5 text-[11px] text-muted-foreground/60">unbound</span>;
  }
  return (
    <span
      className={cn(
        "rounded border px-2 py-0.5 font-mono text-[11px]",
        registered ? "border-border bg-background/60 text-foreground/90" : "border-destructive/40 bg-destructive/5 text-destructive",
      )}
      title={registered ? undefined : "This shortcut couldn't be registered (conflict)"}
    >
      {prettyHotkey(spec)}
    </span>
  );
}

export function Button({
  children,
  onClick,
  variant = "default",
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "default" | "ghost" | "danger";
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "rounded-md px-3.5 py-2 text-sm font-medium transition-colors",
        variant === "default" && "bg-muted/70 text-foreground hover:bg-muted",
        variant === "ghost" && "border border-border text-muted-foreground hover:bg-muted/50 hover:text-foreground",
        variant === "danger" && "border border-destructive/40 text-destructive hover:bg-destructive/10",
      )}
    >
      {children}
    </button>
  );
}

export function Toast({ toast }: { toast: { msg: string; ok: boolean } | null }) {
  return (
    <AnimatePresence>
      {toast && (
        <motion.div
          initial={{ opacity: 0, y: 16, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 16, scale: 0.96 }}
          className={cn(
            "pointer-events-none fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-md border px-4 py-2 text-sm shadow-xl backdrop-blur",
            toast.ok ? "border-border bg-popover/80 text-foreground" : "border-destructive/40 bg-destructive/20 text-destructive-foreground",
          )}
        >
          {toast.msg}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
