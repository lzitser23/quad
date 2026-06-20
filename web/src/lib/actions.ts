// Static metadata for every WindowAction, driving the visual guide previews.
// `key` must match the C# WindowAction enum name exactly.

export type Category = "Halves" | "Quarters" | "Thirds" | "Resize & Center" | "Move";

export interface PreviewRegion {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Glyph = "restore" | "larger" | "smaller" | "nextDisplay" | "prevDisplay" | "missionControl";

export interface ActionMeta {
  key: string;
  display: string;
  defaultHotkey: string;
  category: Category;
  blurb: string;
  regions?: PreviewRegion[];
  glyph?: Glyph;
  cycles?: boolean;
}

const T = 1 / 3;
const TT = 2 / 3;

export const ACTIONS: ActionMeta[] = [
  // Halves
  { key: "LeftHalf", display: "Left Half", defaultHotkey: "Ctrl+Alt+Left", category: "Halves", cycles: true, blurb: "½ → ⅔ → ⅓ on repeat", regions: [{ x: 0, y: 0, w: 0.5, h: 1 }] },
  { key: "RightHalf", display: "Right Half", defaultHotkey: "Ctrl+Alt+Right", category: "Halves", cycles: true, blurb: "½ → ⅔ → ⅓ on repeat", regions: [{ x: 0.5, y: 0, w: 0.5, h: 1 }] },
  { key: "TopHalf", display: "Top Half", defaultHotkey: "Ctrl+Alt+Up", category: "Halves", blurb: "Top 50%", regions: [{ x: 0, y: 0, w: 1, h: 0.5 }] },
  { key: "BottomHalf", display: "Bottom Half", defaultHotkey: "Ctrl+Alt+Down", category: "Halves", blurb: "Bottom 50%", regions: [{ x: 0, y: 0.5, w: 1, h: 0.5 }] },

  // Quarters
  { key: "TopLeftQuarter", display: "Top-Left", defaultHotkey: "Ctrl+Alt+U", category: "Quarters", blurb: "Top-left quarter", regions: [{ x: 0, y: 0, w: 0.5, h: 0.5 }] },
  { key: "TopRightQuarter", display: "Top-Right", defaultHotkey: "Ctrl+Alt+I", category: "Quarters", blurb: "Top-right quarter", regions: [{ x: 0.5, y: 0, w: 0.5, h: 0.5 }] },
  { key: "BottomLeftQuarter", display: "Bottom-Left", defaultHotkey: "Ctrl+Alt+J", category: "Quarters", blurb: "Bottom-left quarter", regions: [{ x: 0, y: 0.5, w: 0.5, h: 0.5 }] },
  { key: "BottomRightQuarter", display: "Bottom-Right", defaultHotkey: "Ctrl+Alt+K", category: "Quarters", blurb: "Bottom-right quarter", regions: [{ x: 0.5, y: 0.5, w: 0.5, h: 0.5 }] },

  // Thirds
  { key: "FirstThird", display: "First Third", defaultHotkey: "Ctrl+Alt+D", category: "Thirds", cycles: true, blurb: "first → center → last", regions: [{ x: 0, y: 0, w: T, h: 1 }] },
  { key: "CenterThird", display: "Center Third", defaultHotkey: "Ctrl+Alt+F", category: "Thirds", blurb: "Middle third", regions: [{ x: T, y: 0, w: T, h: 1 }] },
  { key: "LastThird", display: "Last Third", defaultHotkey: "Ctrl+Alt+G", category: "Thirds", cycles: true, blurb: "last → center → first", regions: [{ x: TT, y: 0, w: T, h: 1 }] },
  { key: "FirstTwoThirds", display: "First Two-Thirds", defaultHotkey: "Ctrl+Alt+E", category: "Thirds", blurb: "Left ⅔", regions: [{ x: 0, y: 0, w: TT, h: 1 }] },
  { key: "LastTwoThirds", display: "Last Two-Thirds", defaultHotkey: "Ctrl+Alt+T", category: "Thirds", blurb: "Right ⅔", regions: [{ x: T, y: 0, w: TT, h: 1 }] },

  // Resize & Center
  { key: "Maximize", display: "Maximize", defaultHotkey: "Ctrl+Alt+Enter", category: "Resize & Center", blurb: "Fill the work area", regions: [{ x: 0, y: 0, w: 1, h: 1 }] },
  { key: "AlmostMaximize", display: "Almost Maximize", defaultHotkey: "", category: "Resize & Center", blurb: "90%, centered", regions: [{ x: 0.06, y: 0.08, w: 0.88, h: 0.84 }] },
  { key: "MaximizeHeight", display: "Maximize Height", defaultHotkey: "", category: "Resize & Center", blurb: "Full height, keep width", regions: [{ x: 0.34, y: 0, w: 0.32, h: 1 }] },
  { key: "Center", display: "Center", defaultHotkey: "Ctrl+Alt+C", category: "Resize & Center", blurb: "Center, keep size", regions: [{ x: 0.26, y: 0.22, w: 0.48, h: 0.56 }] },
  { key: "MakeLarger", display: "Make Larger", defaultHotkey: "Ctrl+Alt++", category: "Resize & Center", blurb: "Grow by a step", glyph: "larger" },
  { key: "MakeSmaller", display: "Make Smaller", defaultHotkey: "Ctrl+Alt+-", category: "Resize & Center", blurb: "Shrink by a step", glyph: "smaller" },
  { key: "Restore", display: "Restore", defaultHotkey: "Ctrl+Alt+Back", category: "Resize & Center", blurb: "Back to pre-snap size", glyph: "restore" },

  // Move
  { key: "NextDisplay", display: "Next Display", defaultHotkey: "Ctrl+Alt+Win+Right", category: "Move", blurb: "Send to right monitor", glyph: "nextDisplay" },
  { key: "PreviousDisplay", display: "Previous Display", defaultHotkey: "Ctrl+Alt+Win+Left", category: "Move", blurb: "Send to left monitor", glyph: "prevDisplay" },
  { key: "MissionControl", display: "Mission Control", defaultHotkey: "Ctrl+Alt+M", category: "Move", blurb: "All windows + desktops (Task View)", glyph: "missionControl" },
];

export const CATEGORIES: Category[] = ["Halves", "Quarters", "Thirds", "Resize & Center", "Move"];

export function metaFor(key: string): ActionMeta | undefined {
  return ACTIONS.find((a) => a.key === key);
}
