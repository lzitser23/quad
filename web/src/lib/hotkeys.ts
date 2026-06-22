// The TS owner of the hotkey-spec grammar: format (prettyHotkey) and capture (tokenFromCode).
// It mirrors the Rust owner in src-tauri/src/hotkeys.rs; the spec string is the seam.

/** Map a browser KeyboardEvent.code to a Quad spec token; null for modifier-only / unknown keys. */
export function tokenFromCode(code: string): string | null {
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

const IS_MAC =
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent || "");

// On macOS the "Win"/Super token IS the Command key; render native Apple symbols so the labels
// read like the rest of the system (⌃⌥⇧⌘ →) instead of Windows names.
const MAC_MODS: Record<string, string> = {
  ctrl: "⌃", control: "⌃",
  alt: "⌥", option: "⌥",
  shift: "⇧",
  win: "⌘", cmd: "⌘", super: "⌘", meta: "⌘",
};
const MAC_MOD_ORDER: Record<string, number> = { "⌃": 0, "⌥": 1, "⇧": 2, "⌘": 3 };

/** macOS key glyphs (arrows, ↩, ⌫, …). */
function macKey(low: string, raw: string): string {
  switch (low) {
    case "left": return "←";
    case "right": return "→";
    case "up": return "↑";
    case "down": return "↓";
    case "enter": case "return": return "↩";
    case "back": case "backspace": return "⌫";
    case "delete": case "del": return "⌦";
    case "escape": case "esc": return "⎋";
    case "space": return "Space";
    case "oemplus": case "plus": return "+";
    case "oemminus": case "minus": return "−";
    case "oemcomma": return ",";
    case "oemperiod": return ".";
    case "oem1": return ";";
    case "oem2": return "/";
    default: return raw.length === 1 ? raw.toUpperCase() : raw.charAt(0).toUpperCase() + raw.slice(1);
  }
}

/** Turn a stored hotkey spec ("Ctrl+Alt+Win+Right") into something readable — "Ctrl + Alt + Win + Right"
 *  on Windows, native "⌃⌥⌘→" on macOS. */
export function prettyHotkey(spec: string): string {
  if (!spec) return "";
  const parts = spec.split("+").map((p) => p.trim()).filter(Boolean);

  if (IS_MAC) {
    const mods: string[] = [];
    let key = "";
    for (const p of parts) {
      const low = p.toLowerCase();
      if (low in MAC_MODS) mods.push(MAC_MODS[low]);
      else key = macKey(low, p);
    }
    mods.sort((a, b) => (MAC_MOD_ORDER[a] ?? 9) - (MAC_MOD_ORDER[b] ?? 9));
    return mods.join("") + key;
  }

  // Windows / other — unchanged.
  return parts
    .map((t) => {
      const low = t.toLowerCase();
      if (low === "oemplus" || low === "plus") return "+";
      if (low === "oemminus" || low === "minus") return "−";
      if (low === "back" || low === "backspace") return "Backspace";
      if (low === "oemcomma") return ",";
      if (low === "oemperiod") return ".";
      if (low === "oem1") return ";";
      if (low === "oem2") return "/";
      return t.length === 1 ? t.toUpperCase() : t.charAt(0).toUpperCase() + t.slice(1);
    })
    .join(" + ");
}
