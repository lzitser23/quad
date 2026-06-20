import { useState, type KeyboardEvent } from "react";
import { cn } from "../lib/utils";
import { prettyHotkey, tokenFromCode } from "../lib/hotkeys";

export function HotkeyInput({
  value,
  registered,
  error,
  onChange,
  onClear,
}: {
  value: string;
  registered: boolean;
  error?: string | null;
  onChange: (spec: string) => void;
  onClear: () => void;
}) {
  const [capturing, setCapturing] = useState(false);

  function onKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      setCapturing(false);
      (e.target as HTMLElement).blur();
      return;
    }
    const key = tokenFromCode(e.code);
    if (!key) return; // wait for a non-modifier key
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (e.metaKey) mods.push("Win");
    onChange([...mods, key].join("+"));
    setCapturing(false);
    (e.target as HTMLElement).blur();
  }

  const empty = !value;
  const state = capturing
    ? "border-accent ring-2 ring-accent/30 text-accent"
    : empty
      ? "border-border text-muted-foreground"
      : registered
        ? "border-border text-foreground"
        : "border-destructive/50 text-destructive";

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        tabIndex={0}
        onFocus={() => setCapturing(true)}
        onBlur={() => setCapturing(false)}
        onKeyDown={onKeyDown}
        title={error ?? undefined}
        className={cn(
          "min-w-[150px] rounded-md border bg-background/60 px-3 py-1.5 text-center text-xs font-medium outline-none transition-all",
          state,
        )}
      >
        {capturing ? "Press keys…" : empty ? "Unbound — click to set" : prettyHotkey(value)}
      </button>
      {!empty && (
        <button
          type="button"
          onClick={onClear}
          className="rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          title="Clear binding"
        >
          ✕
        </button>
      )}
      {!empty && !registered && !capturing && (
        <span className="text-[11px] text-destructive" title={error ?? undefined}>
          conflict
        </span>
      )}
    </div>
  );
}
