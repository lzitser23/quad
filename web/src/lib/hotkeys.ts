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

/** Turn a stored hotkey spec ("Ctrl+Alt+Oemplus") into something readable ("Ctrl + Alt + +"). */
export function prettyHotkey(spec: string): string {
  if (!spec) return "";
  return spec
    .split("+")
    .map((p) => {
      const t = p.trim();
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
