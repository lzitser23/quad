# Tiling geometry is a pure module; winmgr is the imperative shell

The tiling math — halves, thirds, quarters, size-cycling, drag-snap zones, cross-monitor mapping —
lives in `layout` as pure value transforms (`target_rect`, `advance`, `zone`, `map_proportional`),
with no Win32 and no mutable state. `winmgr` is the imperative shell: it resolves the foreground
window and its monitor, calls `layout`, and applies the result with `SetWindowPos`.

The reason: the geometry is the heart of Quad and was previously untestable — entangled with live
HWNDs and mutable cycle state inside one method. Now the interface is the test surface (see
`layout::tests`), and a new action only needs a `match` arm plus a test. Keep new geometry in
`layout` (pure, tested); keep Win32 and per-window state in `winmgr`.
