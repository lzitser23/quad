import { motion } from "framer-motion";
import type { ActionMeta, Glyph as GlyphKind } from "../lib/actions";
import { cn } from "../lib/utils";

const REGION =
  "bg-[linear-gradient(135deg,oklch(var(--accent)/0.95),oklch(var(--accent)/0.7))] shadow-[0_0_14px_-4px_oklch(var(--accent)/0.9)] ring-1 ring-[oklch(0.99_0_0/0.25)]";

export function LayoutPreview({ meta, className }: { meta: ActionMeta; className?: string }) {
  return (
    <div className={cn("relative aspect-[16/10] w-full rounded-md border border-border bg-card p-1.5", className)}>
      <div className="relative h-full w-full overflow-hidden rounded-[3px] bg-background bg-grid">
        {meta.regions?.map((r, i) => (
          <motion.div
            key={i}
            initial={{ opacity: 0, scale: 0.92 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.28, delay: i * 0.05, ease: "easeOut" }}
            className={cn("absolute rounded-[2px]", REGION)}
            style={{
              left: `${r.x * 100}%`,
              top: `${r.y * 100}%`,
              width: `${r.w * 100}%`,
              height: `${r.h * 100}%`,
            }}
          />
        ))}
        {meta.glyph && <Glyph kind={meta.glyph} />}
      </div>
    </div>
  );
}

function Glyph({ kind }: { kind: GlyphKind }) {
  const win = cn("rounded-[2px]", REGION);
  if (kind === "restore") {
    return (
      <div className="absolute inset-0 flex items-center justify-center">
        <div className={cn("flex h-[55%] w-[55%] items-center justify-center", win)}>
          <svg viewBox="0 0 24 24" className="h-1/2 w-1/2 text-accent-foreground" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M3 12a9 9 0 1 0 3-6.7L3 8" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M3 3v5h5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </div>
      </div>
    );
  }
  if (kind === "larger" || kind === "smaller") {
    const out = kind === "larger";
    return (
      <div className="absolute inset-0 flex items-center justify-center">
        <div className={cn("flex items-center justify-center", win)} style={{ width: out ? "48%" : "64%", height: out ? "48%" : "64%" }}>
          <svg viewBox="0 0 24 24" className="h-1/2 w-1/2 text-accent-foreground" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            {out ? <path d="M8 3H3v5M16 3h5v5M8 21H3v-5M16 21h5v-5" /> : <path d="M3 8h5V3M21 8h-5V3M3 16h5v5M21 16h-5v5" />}
          </svg>
        </div>
      </div>
    );
  }
  if (kind === "missionControl") {
    // all windows "popped out" — a few fanned thumbnails, the front one accented
    return (
      <div className="absolute inset-0 flex items-center justify-center">
        <div className="relative h-[64%] w-[70%]">
          <div className="absolute left-0 top-0 h-[52%] w-[52%] rounded-[2px] border border-border bg-muted/50" />
          <div className="absolute right-0 top-[8%] h-[48%] w-[46%] rounded-[2px] border border-border bg-muted/40" />
          <div className={cn("absolute bottom-0 left-[10%] h-[52%] w-[58%]", win)} />
        </div>
      </div>
    );
  }
  // nextDisplay / prevDisplay: two monitors, window highlighted on the target side
  const next = kind === "nextDisplay";
  return (
    <div className="absolute inset-0 flex items-center justify-center gap-2 px-3">
      <div className={cn("flex h-[60%] w-[42%] items-center justify-center rounded-[2px] border border-border", next ? "bg-muted/40" : "")}>
        {!next && <div className={cn("h-3/4 w-3/4", win)} />}
      </div>
      <svg viewBox="0 0 24 24" className="h-4 w-4 shrink-0 text-accent" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        {next ? <path d="M5 12h14M13 6l6 6-6 6" /> : <path d="M19 12H5M11 6l-6 6 6 6" />}
      </svg>
      <div className={cn("flex h-[60%] w-[42%] items-center justify-center rounded-[2px] border border-border", next ? "" : "bg-muted/40")}>
        {next && <div className={cn("h-3/4 w-3/4", win)} />}
      </div>
    </div>
  );
}
