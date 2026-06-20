import { Logo, Stat, Button } from "../components/ui";
import { prettyHotkey } from "../lib/hotkeys";
import { api } from "../lib/bridge";
import type { AppState } from "../lib/types";

export function About({ state, notify }: { state: AppState; notify: (m: string, ok?: boolean) => void }) {
  const failed = state.actions.filter((a) => a.bound && !a.registered);

  return (
    <div className="mx-auto max-w-3xl space-y-7 px-6 py-8">
      <section className="rounded-xl border border-border bg-card/50 p-6">
        <div className="flex items-center gap-3">
          <Logo size={40} />
          <div>
            <div className="text-lg font-semibold text-foreground">Quad</div>
            <div className="text-xs text-muted-foreground">v{state.version}</div>
          </div>
        </div>
        <p className="mt-4 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Keyboard-driven window tiling for Windows, modeled on macOS{" "}
          <a className="text-accent hover:underline" href="https://github.com/rxhanson/Rectangle" target="_blank" rel="noreferrer">
            Rectangle
          </a>
          . Halves, thirds, quarters and size-cycling shortcuts, multi-monitor moves, and drag-to-edge snapping — all
          native Win32 under the hood, with this UI rendered in WebView2.
        </p>
        <div className="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat label="actions" value={state.actions.length} />
          <Stat label="shortcuts active" value={state.registeredCount} />
          <Stat label="conflicts" value={state.failedCount} warn={state.failedCount > 0} />
          <Stat label="drag-snap" value={state.settings.dragSnapEnabled ? "on" : "off"} />
        </div>
      </section>

      {failed.length > 0 && (
        <section className="rounded-xl border border-destructive/30 bg-destructive/[0.08] p-5">
          <h3 className="text-sm font-semibold text-destructive">Some shortcuts couldn't be registered</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            These are claimed by another app or by Windows itself. Rebind them in Settings. Tip: Ctrl+Alt+Arrow is
            frequently reserved by Intel graphics' display-rotation hotkeys — you can disable those in the Intel Graphics
            Command Center, or just pick different keys.
          </p>
          <ul className="mt-3 space-y-1">
            {failed.map((a) => (
              <li key={a.action} className="flex items-center justify-between gap-3 text-xs">
                <span className="text-foreground">{a.display}</span>
                <span className="font-mono text-destructive">{prettyHotkey(a.hotkey)}</span>
                <span className="text-muted-foreground">{a.error}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="flex flex-wrap items-center gap-3">
        <Button onClick={() => api.openSettingsFile()}>Open settings file</Button>
        <Button onClick={() => api.openLog()}>Open log</Button>
        <Button
          variant="danger"
          onClick={async () => {
            notify("Quitting Quad…");
            await api.quit();
          }}
        >
          Quit Quad
        </Button>
      </section>

      <section className="space-y-1 text-[11px] text-muted-foreground/70">
        <div>
          settings: <span className="font-mono text-muted-foreground">{state.settingsPath}</span>
        </div>
        <div>
          log: <span className="font-mono text-muted-foreground">{state.logPath}</span>
        </div>
      </section>
    </div>
  );
}
