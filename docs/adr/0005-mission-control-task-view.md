# Mission Control maps to Windows Task View via synthesized Win+Tab

The Mission Control action opens Windows **Task View** (all windows on the current desktop + the
virtual-desktop strip) — the closest native equivalent of macOS Mission Control. It works by
synthesizing a Win+Tab keystroke with `SendInput` (`winmgr::show_task_view`).

There is no stable public API to open Task View; the documented surface (`IVirtualDesktopManager`)
manages desktops but doesn't show the overview. Synthesizing the shell's own shortcut is the
supported, low-risk path. Because it isn't a per-window action, it dispatches before the
manageable-window check in `execute_on` and returns `None` from `layout::target_rect`.

The injection first releases the Ctrl/Alt the triggering hotkey is still holding — otherwise the
held Alt turns the injected Tab into Alt+Tab (a window switcher) instead of Win+Tab (Task View).
