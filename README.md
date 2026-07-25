<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Quad logo" width="116" />
</p>

<h1 align="center">Quad</h1>

<p align="center">
  <strong>Rectangle-style keyboard-driven window tiling for Windows and macOS.</strong>
</p>

<p align="center">
  <a href="#features">Features</a> |
  <a href="#installation">Installation</a> |
  <a href="#signing">Signing</a> |
  <a href="#quick-start">Quick Start</a> |
  <a href="#development">Development</a> |
  <a href="#architecture">Architecture</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-orange" alt="Platform: Windows | macOS" />
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-engine-DEA584" alt="Rust engine" />
  <img src="https://img.shields.io/badge/React-18-61DAFB" alt="React 18" />
  <img src="https://img.shields.io/badge/TypeScript-5-3178C6" alt="TypeScript 5" />
  <img src="https://github.com/lzitser23/quad/actions/workflows/build.yml/badge.svg" alt="Build status" />
</p>

---

## Overview

**Quad** is a keyboard-driven window tiler for **Windows and macOS**, modeled on macOS [Rectangle](https://github.com/rxhanson/Rectangle): snap windows to halves, thirds, and quarters, maximize / center / restore, cycle sizes by repeating a shortcut, move windows across monitors, and (on Windows) drag a window to a screen edge to snap it.

The window-management engine is native **Rust** — Win32 via the `windows` crate on Windows, the Accessibility / CoreGraphics / AppKit APIs on macOS — with global hotkeys via `tauri-plugin-global-shortcut`. The app's window — a settings screen, a visual shortcut guide, click-to-apply layouts, and status — is a **React** UI rendered by **Tauri 2** (WebView2 on Windows, WKWebView on macOS). It lives in the system tray / menu bar. Settings and the log live in `%APPDATA%\Quad\` on Windows and `~/Library/Application Support/Quad/` on macOS.

---

## Features

- **Tiling shortcuts** — halves, thirds, quarters, maximize, center, with **size-cycling on repeat** (Left Half walks ½ → ⅔ → ⅓, just like Rectangle).
- **Drag-to-edge snapping** — drag a window to an edge → half, a corner → quarter, the top → maximize, with a translucent teal preview that snaps on release.
- **Mission Control** — one shortcut opens Windows **Task View** (all windows on the current desktop + the virtual-desktop strip).
- **Multi-monitor** — move a window to the next / previous display, preserving its relative size and position.
- **Restore** — remembers each window's pre-tiling geometry and brings it back.
- **Click-to-apply** — click any layout in the UI to apply it to your last active window.
- **Live rebinding** — rebind every shortcut from the settings screen; conflicts (e.g. `Ctrl+Alt+Arrow` claimed by Intel graphics) are detected and flagged.
- **Frameless custom chrome** — the dark UI runs edge to edge with its own draggable title bar, while keeping native resize, Aero Snap, and double-click-to-maximize.

---

## Installation

### Download a build

Every push to `main` builds the Windows portable / installer and the macOS universal DMG via [GitHub Actions](https://github.com/lzitser23/quad/actions/workflows/build.yml) and publishes them to the [latest release](https://github.com/lzitser23/quad/releases/latest). They're also attached as artifacts on each workflow run.

| Platform | Asset | Notes |
| --- | --- | --- |
| Windows x64 | `quad.exe` | Portable — run it directly; it starts in the system tray. |
| Windows x64 | `Quad_<version>_x64-setup.exe` | NSIS installer. |
| macOS (universal) | `Quad_<version>_universal.dmg` | Universal (Apple Silicon + Intel) disk image. |
| macOS (universal) | `quad-macos-universal.app.zip` | Bare `.app` bundle — used by the in-app self-updater. |

**Windows:** the binaries are **unsigned** unless the maintainer has configured a code-signing certificate (see [Signing](#signing)), so SmartScreen may warn on first run — choose **More info → Run anyway**. Quad uses the Microsoft Edge **WebView2** runtime (preinstalled on Windows 11 and most updated Windows 10).

**macOS:** open the `.dmg` and drag **Quad** to `/Applications`. Unless the build was signed and notarized with a Developer ID (see [Signing](#signing)), Gatekeeper warns on first launch — **right-click → Open**, then confirm, or run `xattr -dr com.apple.quarantine /Applications/Quad.app` to clear the quarantine flag.

To move other apps' windows, macOS requires **Accessibility** permission: on first run Quad opens **System Settings → Privacy & Security → Accessibility** — enable **Quad** there, then **relaunch** the app (the permission only takes effect on a fresh launch). Until it's granted, Quad shows an in-app banner and its tiling actions no-op. macOS 11+ is required.

### Build from source

See [Development](#development).

---

## Signing

Release binaries are **unsigned by default**: CI produces working artifacts with no (Windows) / ad-hoc
(macOS) signature, so first-run OS warnings are expected. Signing turns on automatically once the
maintainer adds the corresponding repository secrets — no code changes needed.

### Windows (Authenticode)

Set `WINDOWS_CERTIFICATE` (a base64-encoded `.pfx`) and `WINDOWS_CERTIFICATE_PASSWORD`. When present,
CI Authenticode-signs the portable `quad.exe` and the NSIS installer (SHA-256, RFC-3161 timestamped),
so SmartScreen stops warning. Without them the binaries ship unsigned — choose **More info → Run
anyway**.

> The workflow signs the two artifacts users download and launch (the portable exe and the installer).
> The `quad.exe` the installer unpacks is signed at build time only if you additionally configure
> Tauri's own `bundle.windows` signing. An Authenticode or Azure Trusted Signing certificate must be
> procured separately — none is committed to the repo.

### macOS (Developer ID + notarization)

Set `APPLE_SIGNING_IDENTITY`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`,
`APPLE_PASSWORD`, and `APPLE_TEAM_ID`. When present, CI signs with your Developer ID and notarizes the
DMG; otherwise the build is ad-hoc-signed and Gatekeeper warns on first launch (see
[Installation](#installation)).

---

## Quick Start

1. Run `quad.exe` (or the installer). Quad starts in the system tray — the offset-tile mark with the teal accent tile.
2. Snap the focused window with a shortcut (`Ctrl+Alt` mirrors Rectangle's `Ctrl+Option`):

   | Action | Shortcut |
   | --- | --- |
   | Left / Right Half | `Ctrl+Alt+←` / `→` *(repeat to cycle ½ → ⅔ → ⅓)* |
   | Top / Bottom Half | `Ctrl+Alt+↑` / `↓` |
   | Quarters | `Ctrl+Alt+U` / `I` / `J` / `K` |
   | First / Center / Last Third | `Ctrl+Alt+D` / `F` / `G` |
   | Maximize · Center · Restore | `Ctrl+Alt+Enter` · `Ctrl+Alt+C` · `Ctrl+Alt+Backspace` |
   | Next / Previous Display | `Ctrl+Alt+Win+→` / `←` |
   | **Mission Control** | `Ctrl+Alt+M` |

3. Or **drag** a window to a screen edge / corner to snap it.
4. Open the window from the tray (or **click** any layout there to apply it to your last window), and **rebind** anything in Settings — all bindings are editable, or via `%APPDATA%\Quad\settings.json`.

---

## Stack

| Layer | Choice |
| --- | --- |
| Shell & packaging | [Tauri 2](https://tauri.app) |
| Native engine | Rust 2021 — [`windows`](https://crates.io/crates/windows) 0.58, [`tauri-plugin-global-shortcut`](https://crates.io/crates/tauri-plugin-global-shortcut) 2 |
| UI | React 18 · TypeScript 5 · [Vite](https://vitejs.dev) 5 |
| Styling | Tailwind 3 · Framer Motion 11 · [Aceternity UI](https://ui.aceternity.com) components · JetBrains Mono |
| Settings & autostart | `serde_json` + `winreg` (HKCU `Run`) |

---

## Development

### Prerequisites

- **Rust** (stable — MSVC toolchain on Windows, the default toolchain on macOS) and **Node.js 20+**.
- **Windows:** the Microsoft Edge **WebView2** runtime (preinstalled on Windows 11 / updated Windows 10).
- **macOS:** Xcode Command Line Tools (`xcode-select --install`); macOS 11+.

### Commands

```bash
git clone https://github.com/lzitser23/quad.git
cd quad
npm install            # Tauri CLI
npm --prefix web install   # frontend deps
```

```bash
npm run tauri dev      # hot-reloading dev window
npm run tauri build    # portable quad.exe + NSIS installer in src-tauri/target/release
cargo test --manifest-path src-tauri/Cargo.toml   # the pure engine's unit tests
```

> A plain `cargo build` runs in dev mode (loads the Vite dev URL). Production builds must go through the Tauri CLI (`npm run tauri build`) so the frontend is embedded.

### Project Structure

```text
quad/
|-- src-tauri/   # Rust engine: layout, winmgr, hotkeys, ipc, tray, state, native
|-- web/         # React + Tailwind + Framer Motion UI
|-- docs/adr/    # architecture decision records
|-- .github/     # CI: build the portable on merge to main
`-- CONTEXT.md   # domain glossary
```

---

## Architecture

The engine is a **pure core under a thin imperative shell**: `src-tauri/src/layout.rs` computes every tiling rect (halves/thirds/quarters, size-cycling, snap zones) as pure value transforms with no Win32, so the geometry is unit-tested directly; `winmgr.rs` resolves the foreground window and its monitor, calls `layout`, and applies the result with `SetWindowPos`. Hotkeys, the IPC contract, the tray, and shared state each live in their own module; the React UI talks to Rust through `#[tauri::command]`s and `emit` events, mirrored in `web/src/lib/`.

- **[CONTEXT.md](CONTEXT.md)** — the domain glossary (Action, work area, snap zone, hotkey spec…) and the module map.
- **[docs/adr/](docs/adr/)** — architecture decision records: the Tauri/Rust choice, the pure-layout split, the hotkey-spec seam, and the Mission Control → Task View mapping.

---

## Acknowledgments

- [Rectangle](https://github.com/rxhanson/Rectangle) — the macOS app Quad is modeled on.
- [Tauri](https://tauri.app) and the [`windows`](https://github.com/microsoft/windows-rs) crate.
- [Aceternity UI](https://ui.aceternity.com) — UI components.
