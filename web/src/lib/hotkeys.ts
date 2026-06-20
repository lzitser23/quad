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
