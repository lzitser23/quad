import { useEffect, useState } from "react";
import { Toggle } from "../components/Toggle";
import { HotkeyInput } from "../components/HotkeyInput";
import { Button } from "../components/ui";
import { api } from "../lib/bridge";
import type { AppState, SettingsPatch } from "../lib/types";

export function Settings({
  state,
  setState,
  notify,
}: {
  state: AppState;
  setState: (s: AppState) => void;
  notify: (m: string, ok?: boolean) => void;
}) {
  const s = state.settings;

  async function patch(p: SettingsPatch) {
    try {
      setState(await api.updateSettings(p));
    } catch (e: any) {
      notify(e?.message ?? "Failed to save", false);
    }
  }

  async function setKey(action: string, spec: string) {
    try {
      setState(await api.setHotkey(action, spec));
    } catch (e: any) {
      notify(e?.message ?? "Invalid hotkey", false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-9 px-6 py-8">
      <section className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Behavior</h2>
        <Toggle
          checked={s.dragSnapEnabled}
          onChange={(v) => patch({ dragSnapEnabled: v })}
          label="Drag windows to screen edges to snap"
          description="Edge → half, corner → quarter, top → maximize. Snaps on release."
        />
        <Toggle
          checked={s.showSnapPreview}
          onChange={(v) => patch({ showSnapPreview: v })}
          label="Show the translucent snap preview"
          description="Highlights where the window will land while you drag."
        />
        <Toggle
          checked={s.startWithWindows}
          onChange={(v) => patch({ startWithWindows: v })}
          label="Start Quad when I sign in"
        />
      </section>

      <section className="space-y-4">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Tuning</h2>
        <Slider label="Snap edge sensitivity" suffix="px" min={5} max={60} value={s.snapEdgeThresholdPx} onCommit={(v) => patch({ snapEdgeThresholdPx: v })} />
        <Slider label="Resize step (Make Larger / Smaller)" suffix="px" min={10} max={200} value={s.resizeStepPx} onCommit={(v) => patch({ resizeStepPx: v })} />
        <Slider label="Gap between tiled windows" suffix="px" min={0} max={40} value={s.gapPx} onCommit={(v) => patch({ gapPx: v })} />
      </section>

      <section className="space-y-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Keyboard shortcuts</h2>
          <Button variant="ghost" onClick={() => api.openSettingsFile()}>
            Open settings file
          </Button>
        </div>
        <p className="text-xs text-muted-foreground">
          Click a binding and press your combination. <span className="text-foreground">Ctrl+Alt</span> mirrors
          Rectangle's Ctrl+Option. If a binding shows <span className="text-destructive">conflict</span>, it's taken by another
          app or Windows (Ctrl+Alt+Arrow is often claimed by Intel graphics).
        </p>
        <div className="divide-y divide-border overflow-hidden rounded-lg border border-border bg-card/50">
          {state.actions.map((a) => (
            <div key={a.action} className="flex items-center justify-between gap-3 px-4 py-2.5">
              <span className="text-sm text-foreground">{a.display}</span>
              <HotkeyInput
                value={a.hotkey}
                registered={a.registered}
                error={a.error}
                onChange={(spec) => setKey(a.action, spec)}
                onClear={() => setKey(a.action, "")}
              />
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function Slider({
  label,
  suffix,
  min,
  max,
  value,
  onCommit,
}: {
  label: string;
  suffix?: string;
  min: number;
  max: number;
  value: number;
  onCommit: (v: number) => void;
}) {
  const [local, setLocal] = useState(value);
  useEffect(() => setLocal(value), [value]);

  return (
    <div className="rounded-lg border border-border bg-card/50 px-4 py-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-sm text-foreground">{label}</span>
        <span className="font-mono text-xs text-muted-foreground">
          {local}
          {suffix}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        value={local}
        onChange={(e) => setLocal(Number(e.target.value))}
        onMouseUp={() => onCommit(local)}
        onTouchEnd={() => onCommit(local)}
        onKeyUp={() => onCommit(local)}
        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-muted accent-[oklch(var(--accent))]"
      />
    </div>
  );
}
