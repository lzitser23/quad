import { useMemo } from "react";
import { Spotlight } from "../components/aceternity/Spotlight";
import { SpotlightCard } from "../components/aceternity/SpotlightCard";
import { LayoutPreview } from "../components/LayoutPreview";
import { Kbd, Stat } from "../components/ui";
import { ACTIONS, CATEGORIES } from "../lib/actions";
import { api } from "../lib/bridge";
import type { ActionDto, AppState } from "../lib/types";

export function Guide({ state, notify }: { state: AppState; notify: (m: string, ok?: boolean) => void }) {
  const byKey = useMemo(() => {
    const map: Record<string, ActionDto> = {};
    for (const a of state.actions) map[a.action] = a;
    return map;
  }, [state]);

  async function apply(key: string, display: string) {
    try {
      const r = await api.applyAction(key);
      notify(r.ok ? `Applied ${display}` : r.message, r.ok);
    } catch (e: any) {
      notify(e?.message ?? "Failed to apply", false);
    }
  }

  return (
    <div>
      <section className="relative overflow-hidden border-b border-border bg-grid px-6 py-14">
        <Spotlight className="-top-40 left-10 md:-top-20 md:left-60" fill="oklch(0.72 0.13 195)" />
        <div className="relative z-10 mx-auto max-w-5xl">
          <h1 className="bg-gradient-to-b from-foreground to-muted-foreground bg-clip-text text-4xl font-bold tracking-tight text-transparent">
            Snap any window, instantly
          </h1>
          <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted-foreground">
            Press a keyboard shortcut, drag a window to a screen edge, or just click a layout below to apply it to
            your last active window. Repeating a half or third <span className="text-foreground">cycles its size</span>,
            just like Rectangle.
          </p>
          <div className="mt-6 flex flex-wrap gap-2">
            <Stat label="shortcuts active" value={state.registeredCount} />
            {state.failedCount > 0 && <Stat label="conflicts" value={state.failedCount} warn />}
            <Stat label="drag-snap" value={state.settings.dragSnapEnabled ? "on" : "off"} />
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-7xl space-y-10 px-6 py-8">
        {CATEGORIES.map((cat) => (
          <div key={cat}>
            <h2 className="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">{cat}</h2>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
              {ACTIONS.filter((a) => a.category === cat).map((a) => {
                const live = byKey[a.key];
                return (
                  <SpotlightCard key={a.key} onClick={() => apply(a.key, a.display)} className="p-3">
                    <LayoutPreview meta={a} />
                    <div className="mt-3">
                      <div className="text-sm font-semibold text-foreground">{a.display}</div>
                      <div className="text-[11px] text-muted-foreground">{a.blurb}</div>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-1.5">
                      <Kbd
                        spec={live?.hotkey ?? a.defaultHotkey}
                        bound={live ? live.bound : a.defaultHotkey.length > 0}
                        registered={live ? live.registered : a.defaultHotkey.length > 0}
                      />
                      {a.cycles && (
                        <span className="rounded bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">cycles</span>
                      )}
                      <span className="ml-auto text-[10px] text-muted-foreground/60 opacity-0 transition-opacity group-hover:opacity-100">
                        click to apply
                      </span>
                    </div>
                  </SpotlightCard>
                );
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
