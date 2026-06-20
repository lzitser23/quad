import type { CSSProperties } from "react";
import { api } from "../lib/bridge";

// Thin invisible drag handles around the window perimeter. Because the WebView2 control covers
// the whole client area, native resize grips are hidden — so we detect edge drags here and hand
// off to the native resize loop (which keeps Aero Snap and live resizing).
const T = 6; // edge thickness
const C = 12; // corner size

type Edge = "top" | "bottom" | "left" | "right" | "topleft" | "topright" | "bottomleft" | "bottomright";

const handles: { edge: Edge; style: CSSProperties; cursor: string }[] = [
  { edge: "top", style: { top: 0, left: C, right: C, height: T }, cursor: "ns-resize" },
  { edge: "bottom", style: { bottom: 0, left: C, right: C, height: T }, cursor: "ns-resize" },
  { edge: "left", style: { left: 0, top: C, bottom: C, width: T }, cursor: "ew-resize" },
  { edge: "right", style: { right: 0, top: C, bottom: C, width: T }, cursor: "ew-resize" },
  { edge: "topleft", style: { top: 0, left: 0, width: C, height: C }, cursor: "nwse-resize" },
  { edge: "topright", style: { top: 0, right: 0, width: C, height: C }, cursor: "nesw-resize" },
  { edge: "bottomleft", style: { bottom: 0, left: 0, width: C, height: C }, cursor: "nesw-resize" },
  { edge: "bottomright", style: { bottom: 0, right: 0, width: C, height: C }, cursor: "nwse-resize" },
];

export function ResizeBorders({ disabled }: { disabled?: boolean }) {
  if (disabled) return null;
  return (
    <>
      {handles.map((h) => (
        <div
          key={h.edge}
          onMouseDown={(e) => {
            if (e.button !== 0) return;
            e.preventDefault();
            api.windowResize(h.edge);
          }}
          style={{ position: "fixed", zIndex: 60, cursor: h.cursor, ...h.style }}
        />
      ))}
    </>
  );
}
