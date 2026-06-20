# WinRect

**Rectangle for Windows.** A keyboard-driven window tiler modeled on macOS
[Rectangle](https://github.com/rxhanson/Rectangle): snap windows to halves, thirds, quarters,
maximize/center/restore, cycle sizes by repeating a shortcut, move windows across monitors, and
drag a window to a screen edge to snap it.

Built with **Tauri 2** — the native window-management engine is **Rust** (Win32 via the `windows`
crate; global hotkeys via `tauri-plugin-global-shortcut`), and the app's window is a **React +
Tailwind + Framer Motion** UI using [Aceternity UI](https://ui.aceternity.com/) components, rendered
in WebView2. It lives in the system tray.

The window is **frameless with custom chrome** (Tauri `decorations: false`): the dark UI runs edge
to edge with its own draggable title bar and minimize / maximize / close buttons, while keeping
native resize, Aero Snap, maximize, and the window shadow.

**Design system:** a sibling-project-inspired token set — **JetBrains Mono** throughout, OKLCH neutral
surfaces with a **teal** accent, squared-off corners, and thin token-driven scrollbars. Tokens are
CSS variables in `web/src/index.css` mapped through `web/tailwind.config.js`, so the whole UI
re-themes from one place.

---

## Quick start

```powershell
# Requires: Rust (stable, MSVC) + Node.js. End users just run the installer.
npm install                # root: Tauri CLI
npm --prefix web install   # frontend deps
npm run tauri build
# → installer:  src-tauri\target\release\bundle\nsis\WinRect_0.1.0_x64-setup.exe
# → bare exe:   src-tauri\target\release\winrect.exe   (~6.5 MB)
```

Run the installer (or the bare exe). WinRect starts in the system tray (the teal split-window icon).
Right-click the tray icon for the menu, or double-click it / choose **Open WinRect** for the UI.
`winrect.exe --open` launches with the window already open.

Dev loop: `npm run tauri dev` (hot-reloads the React UI in the real window).

> **WebView2 runtime:** Tauri uses Microsoft's Edge WebView2 runtime on Windows (preinstalled on
> Windows 11 and most updated Windows 10). Your shortcuts and drag-snap work regardless — it's only
> for the settings window.

---

## Default shortcuts

`Ctrl+Alt` mirrors Rectangle's `Ctrl+Option`. All bindings are editable in **Settings** (or
`%APPDATA%\WinRect\settings.json`).

| Action | Shortcut | Notes |
|---|---|---|
| Left / Right Half | `Ctrl+Alt+←` / `→` | Repeat to cycle **½ → ⅔ → ⅓** |
| Top / Bottom Half | `Ctrl+Alt+↑` / `↓` | |
| Quarters | `Ctrl+Alt+U` / `I` / `J` / `K` | TL / TR / BL / BR |
| First / Center / Last Third | `Ctrl+Alt+D` / `F` / `G` | First & Last cycle through thirds |
| First / Last Two-Thirds | `Ctrl+Alt+E` / `T` | |
| Maximize | `Ctrl+Alt+Enter` | Fills the work area (not OS-maximize) |
| Center | `Ctrl+Alt+C` | Keeps size |
| Restore | `Ctrl+Alt+Backspace` | Back to pre-snap geometry |
| Make Larger / Smaller | `Ctrl+Alt++` / `-` | |
| Next / Previous Display | `Ctrl+Alt+Win+→` / `←` | |
| Almost Maximize / Maximize Height | *(unbound)* | Assign in Settings |

**Drag-to-snap:** drag a window to a screen edge → half, a corner → quarter, the top → maximize.
A translucent teal preview shows where it'll land; it snaps on release.

**Click-to-apply:** in the **Shortcuts** tab, click any layout to apply it to your last active window.

---

## ⚠️ Ctrl+Alt+Arrow conflicts

On many Windows 10 PCs, **Intel graphics drivers reserve `Ctrl+Alt+Arrow`** for screen rotation, so
those four hotkeys may not register. WinRect detects registration conflicts and flags them as
**conflict** in Settings / Status. Fix by disabling the Intel hotkeys (Intel Graphics Command Center
→ *System* → *Hotkeys*) or by rebinding those actions in Settings.

---

## Notes & limitations

- **Elevated windows:** WinRect runs un-elevated and can't move windows owned by elevated (admin)
  processes — Windows blocks it. Run WinRect as admin if you need that.
- **Aero Snap:** WinRect's drag-snap applies on mouse-release, so it generally wins over Windows'
  built-in Aero Snap. If you see fighting, turn off *Settings → System → Multitasking → Snap windows*,
  or disable WinRect's drag-snap from the tray menu.
- **Gaps:** set a pixel gap between tiled windows in Settings (default 0 = flush, like Rectangle).

Settings and log live in `%APPDATA%\WinRect\`.

---

## Architecture

```
src-tauri/                     Tauri (Rust) backend
  src/app.rs                   Tauri builder, commands, tray, hotkey registration, shared state
  src/winmgr.rs                monitors + window manager (DWM-flush, size cycling, restore, multi-monitor)
  src/native.rs                worker thread: WinEvent hooks (foreground + drag-snap) + preview overlay
  src/settings.rs              JSON settings, autostart (HKCU Run), logging
  src/actions.rs               WindowAction table + default hotkeys
  tauri.conf.json              frameless window, tray, bundle config
web/                           React + Tailwind + Framer Motion + Aceternity UI
  src/lib/bridge.ts            invoke()/listen() + window API (drag/resize/min/max/close)
legacy-dotnet/                 the previous .NET 8 / WebView2 implementation (kept for reference)
```

- **Hotkeys:** `tauri-plugin-global-shortcut` registers each binding; failures (conflicts) are
  reported per-action.
- **Window control:** native Win32 `SetWindowPos` with DWM invisible-border compensation, per-monitor
  work areas, and DWM extended-frame-bounds for flush positioning.
- **Custom chrome:** Tauri `decorations: false`; the React title bar drives `startDragging()` /
  `startResizeDragging()` and the window controls.
- **IPC:** Rust `#[tauri::command]`s (`get_state`, `update_settings`, `set_hotkey`, `apply_action`, …)
  and `emit("state", …)` events; the React `api` wrapper mirrors the previous bridge surface.

---

## Credits

Inspired by [Rectangle](https://github.com/rxhanson/Rectangle) by Ryan Hanson.
UI built with [Aceternity UI](https://ui.aceternity.com/) components; design tokens inspired by the
`sibling-project` project's monospace aesthetic.
