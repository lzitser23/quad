import { useState, type KeyboardEvent } from "react";
import { cn } from "../lib/utils";
import { prettyHotkey } from "../lib/hotkeys";

/** Maps a browser KeyboardEvent's main key to a Quad hotkey token (null = modifier only). */
function mapKey(e: KeyboardEvent): string | null {
  const code = e.code;
  if (code.startsWith("Key")) return code.slice(3); // KeyA → A
  if (code.startsWith("Digit")) return code.slice(5); // Digit1 → 1
  if (/^F\d{1,2}$/.test(code)) return code; // F1..F12
  switch (code) {
    case "ArrowLeft": return "Left";
    case "ArrowRight": return "Right";
    case "ArrowUp": return "Up";
    case "ArrowDown": return "Down";
    case "Enter":
    case "NumpadEnter": return "Enter";
    case "Backspace": return "Back";
    case "Delete": return "Delete";
    case "Insert": return "Insert";
    case "Home": return "Home";
    case "End": return "End";
    case "PageUp": return "PageUp";
    case "PageDown": return "PageDown";
    case "Space": return "Space";
    case "Equal":
    case "NumpadAdd": return "Oemplus";
    case "Minus":
    case "NumpadSubtract": return "OemMinus";
    case "Comma": return "OemComma";
    case "Period": return "OemPeriod";
    case "Semicolon": return "Oem1";
    case "Slash": return "Oem2";
    case "BracketLeft": return "Oem4";
    case "BracketRight": return "Oem6";
    default: return null;
  }
}

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
    const key = mapKey(e);
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
