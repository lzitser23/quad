# Quad

Keyboard-driven window tiling for Windows, modeled on macOS Rectangle. A Tauri app: a Rust native
engine moves windows; a React/WebView2 UI configures it. This file is the ubiquitous language —
use these terms in code, commits, and reviews.

## Language

### Window placement

**Action**:
One window-management command — Left Half, Maximize, Restore, Mission Control, … The unit a hotkey
or a click triggers.
_Avoid_: command, operation, gesture

**Work area**:
A monitor's usable region with the taskbar excluded. Tiling targets are computed to fill it.
_Avoid_: screen, desktop, monitor rect

**Tiling**:
Placing a window's visible frame into a rect computed within a work area.
_Avoid_: positioning, arranging

**Cycle**:
The ordered size/position sequence a *repeated* action steps through — Left Half walks ½ → ⅔ → ⅓;
First Third walks first → center → last.
_Avoid_: rotation, toggle

**Restore geometry**:
The pre-tiling rect remembered per window, so the Restore action returns the window to where it was
before Quad first moved it.
_Avoid_: undo, original position

**Visible rect**:
A window's true frame from DWM extended bounds — excludes the invisible resize border, so tiles sit
flush to edges.
_Avoid_: window rect (that one *includes* the border)

### Snapping

**Snap zone**:
The work-area rect that a drag near a screen edge or corner maps to (edge → half, corner → quarter,
top → maximize).
_Avoid_: drop zone, region

**Drag-snap**:
Snapping a window by dragging it to a screen edge, as opposed to by hotkey. Distinct from Windows'
own Aero Snap.
_Avoid_: aero snap

### Windows & desktops

**Manageable window**:
A top-level window Quad will move — visible, captioned or sizable, not minimized, cloaked, or a
shell window.
_Avoid_: valid window, real window

**Mission Control**:
Quad's action that opens Windows **Task View** (every window on the current desktop plus the
virtual-desktop strip) by synthesizing Win+Tab.
_Avoid_: task switcher, expose, overview

### Bindings

**Hotkey spec**:
The string form of a binding — `"Ctrl+Alt+Oemplus"`, `"Ctrl+Alt+Win+Right"`. The contract shared
between the Rust parser and the TS capture/format; the cross-language seam.
_Avoid_: shortcut string, keybind, accelerator

## Modules

The native engine is a pure core under a thin imperative shell (see `docs/adr/0002`):

- **`layout`** — pure tiling geometry, cycling, and snap-zone math. No Win32, no state. The test
  surface (`src-tauri/src/layout.rs`).
- **`winmgr`** — the imperative shell: monitor enumeration, Win32 window I/O, per-window cycle/restore
  state. Resolves windows/monitors, calls `layout`, applies the result.
- **`hotkeys`** — the hotkey-spec grammar and global-shortcut registration (see `docs/adr/0003`).
- **`ipc`** — the Rust↔JS contract: DTOs, state snapshots/events, and Tauri command handlers.
- **`tray`** — the system-tray menu and its events.
- **`state`** — the process-wide shared singletons (settings, window manager, registration table).
- **`native`** — the Win32 worker thread: WinEvent hooks (foreground tracking + drag-snap) and the
  preview-overlay window.

The UI mirrors the IPC contract in `web/src/lib/{types.ts,bridge.ts}` and the hotkey-spec grammar in
`web/src/lib/hotkeys.ts`.
