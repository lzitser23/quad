import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppState, ApplyResult, SettingsPatch } from "./types";
import { ACTIONS } from "./actions";

export function isTauri(): boolean {
  return typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || (window as any).isTauri === true);
}

/** Invoke a Rust command, or fall back to a browser mock for `npm run dev`. */
function cmd<T>(name: string, args: any, mock: () => T): Promise<T> {
  if (!isTauri()) return Promise.resolve(mock());
  return invoke<T>(name, args);
}

/** Subscribe to a Rust-emitted event. Returns a synchronous unsubscribe. */
export function onEvent(event: string, handler: (payload: any) => void): () => void {
  if (!isTauri()) return () => {};
  let un: UnlistenFn | null = null;
  let cancelled = false;
  listen(event, (e) => handler((e as any).payload)).then((f) => {
    if (cancelled) f();
    else un = f;
  });
  return () => {
    cancelled = true;
    if (un) un();
  };
}

/** Track window maximized state (for the custom title bar's restore icon). */
export function watchMaximized(cb: (maximized: boolean) => void): () => void {
  if (!isTauri()) {
    cb(false);
    return () => {};
  }
  const w = getCurrentWindow();
  w.isMaximized().then(cb).catch(() => {});
  let un: UnlistenFn | null = null;
  let cancelled = false;
  w.onResized(() => w.isMaximized().then(cb).catch(() => {})).then((f) => {
    if (cancelled) f();
    else un = f;
  });
  return () => {
    cancelled = true;
    if (un) un();
  };
}

const EDGE_TO_DIR: Record<string, string> = {
  left: "West",
  right: "East",
  top: "North",
  bottom: "South",
  topleft: "NorthWest",
  topright: "NorthEast",
  bottomleft: "SouthWest",
  bottomright: "SouthEast",
};

export const api = {
  getState: () => cmd<AppState>("get_state", undefined, mockState),
  updateSettings: (patch: SettingsPatch) => cmd<AppState>("update_settings", { patch }, () => mockUpdate(patch)),
  setHotkey: (action: string, spec: string) => cmd<AppState>("set_hotkey", { action, spec }, () => mockSetHotkey(action, spec)),
  applyAction: (action: string) =>
    cmd<ApplyResult>("apply_action", { action }, () => ({ ok: true, message: `(preview) ${action}` })),
  openLog: () => cmd("open_log", undefined, () => undefined),
  openSettingsFile: () => cmd("open_settings_file", undefined, () => undefined),
  quit: () => cmd("quit_app", undefined, () => undefined),

  // Custom window chrome → Tauri window API
  windowMinimize: () => (isTauri() ? getCurrentWindow().minimize() : Promise.resolve()),
  windowToggleMaximize: () => (isTauri() ? getCurrentWindow().toggleMaximize() : Promise.resolve()),
  windowClose: () => (isTauri() ? getCurrentWindow().hide() : Promise.resolve()),
  windowDrag: () => (isTauri() ? getCurrentWindow().startDragging() : Promise.resolve()),
  windowResize: (edge: string) =>
    isTauri() ? getCurrentWindow().startResizeDragging(EDGE_TO_DIR[edge] as any) : Promise.resolve(),
};

// ---- Browser dev mock (so `npm run dev` renders without the Tauri host) -----

let mock: AppState | null = null;

function mockState(): AppState {
  if (mock) return mock;
  const actions = ACTIONS.map((a) => ({
    action: a.key,
    display: a.display,
    defaultHotkey: a.defaultHotkey,
    hotkey: a.defaultHotkey,
    bound: a.defaultHotkey.length > 0,
    registered: a.defaultHotkey.length > 0,
    error: null as string | null,
  }));
  mock = {
    version: "0.1.0 (browser preview)",
    settings: {
      dragSnapEnabled: true,
      startWithWindows: false,
      showSnapPreview: true,
      snapEdgeThresholdPx: 20,
      resizeStepPx: 30,
      gapPx: 0,
    },
    actions,
    registeredCount: actions.filter((a) => a.registered).length,
    failedCount: 0,
    settingsPath: "%APPDATA%\\WinRect\\settings.json",
    logPath: "%APPDATA%\\WinRect\\winrect.log",
  };
  return mock;
}

function mockUpdate(patch: SettingsPatch): AppState {
  const s = mockState();
  s.settings = { ...s.settings, ...patch };
  return s;
}

function mockSetHotkey(action: string, spec: string): AppState {
  const s = mockState();
  const a = s.actions.find((x) => x.action === action);
  if (a) {
    a.hotkey = spec;
    a.bound = !!spec;
    a.registered = !!spec;
  }
  s.registeredCount = s.actions.filter((x) => x.registered).length;
  return s;
}
