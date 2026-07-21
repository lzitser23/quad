import { useEffect, useRef, useState } from "react";
import { api, onEvent, watchMaximized } from "./lib/bridge";
import { checkForUpdate, dismissUpdate, type ReleaseUpdate } from "./lib/updateCheck";
import { downloadAndInstall, takeUpdateRecoveryError } from "./lib/updater";
import type { AppState } from "./lib/types";
import { Tabs, type TabItem } from "./components/aceternity/Tabs";
import { Logo, Loading, Toast } from "./components/ui";
import { WindowControls } from "./components/WindowControls";
import { ResizeBorders } from "./components/ResizeBorders";
import { Guide } from "./screens/Guide";
import { Settings } from "./screens/Settings";
import { About } from "./screens/About";

type TabId = "guide" | "settings" | "about";

const TABS: TabItem[] = [
  { id: "guide", label: "Shortcuts" },
  { id: "settings", label: "Settings" },
  { id: "about", label: "Status" },
];

export default function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [tab, setTab] = useState<TabId>("guide");
  const [maximized, setMaximized] = useState(false);
  const [toast, setToast] = useState<{ msg: string; ok: boolean } | null>(null);
  const [update, setUpdate] = useState<ReleaseUpdate | null>(null);
  const toastTimer = useRef<number | undefined>(undefined);

  useEffect(() => {
    api.getState().then(setState).catch(() => {});
    const offState = onEvent("state", (s: AppState) => setState(s));
    const offWin = watchMaximized(setMaximized);
    return () => {
      offState();
      offWin();
    };
  }, []);

  // Once the running version is known, ask GitHub whether a newer release is
  // out. Silent on every failure (offline, rate-limited, no releases yet).
  // A failed swap from a previous update is the exception: reported once.
  const version = state?.version;
  useEffect(() => {
    if (!version) return;
    void takeUpdateRecoveryError().then((error) => {
      if (error) notify(error, false);
    });
    checkForUpdate(version).then(setUpdate, () => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [version]);

  // Progress text while a self-update runs; the pill shows it and goes inert.
  const [updating, setUpdating] = useState<string | null>(null);
  async function runSelfUpdate(u: ReleaseUpdate) {
    if (!u.asset) return;
    setUpdating("downloading…");
    try {
      await downloadAndInstall(u.asset, (p) => {
        if (p.phase === "downloading") {
          const pct = p.totalBytes
            ? ` ${Math.round((p.downloadedBytes / p.totalBytes) * 100)}%`
            : "";
          setUpdating(`downloading…${pct}`);
        } else if (p.phase === "verifying") {
          setUpdating("verifying…");
        } else {
          setUpdating("restarting…");
        }
      });
      // On success apply_update exits the app — anything after is failure-only.
    } catch (e) {
      setUpdating(null);
      notify(`Update failed: ${String(e)}`, false);
    }
  }

  function notify(msg: string, ok = true) {
    setToast({ msg, ok });
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 2600);
  }

  return (
    <div className="flex h-full select-none flex-col">
      <header className="relative z-20 flex items-stretch justify-between gap-4 border-b border-border bg-card/70 pl-6 backdrop-blur">
        {/* Full-bleed drag layer: Tauri handles dragging + double-click-to-maximize.
            Non-interactive content above sets pointer-events:none so clicks fall through here. */}
        <div data-tauri-drag-region className="absolute inset-0" aria-hidden="true" />

        <div className="pointer-events-none relative flex items-center gap-3 py-3">
          <Logo size={28} />
          <div className="text-sm font-semibold tracking-tight text-foreground">Quad</div>
        </div>

        <div className="relative flex items-center py-3">
          <Tabs tabs={TABS} active={tab} onChange={(id) => setTab(id as TabId)} />
        </div>

        <div className="relative flex items-center gap-3">
          {update && (
            <button
              className="rounded-full border border-accent/40 bg-accent/10 px-2.5 py-0.5 text-[11px] text-accent transition-colors hover:bg-accent/20 disabled:pointer-events-none"
              title={update.asset ? "Download and install the update" : "Open the release page"}
              disabled={updating !== null}
              onClick={() => {
                dismissUpdate(update.version);
                if (update.asset) {
                  void runSelfUpdate(update);
                } else {
                  setUpdate(null);
                  api.openUrl(update.url);
                }
              }}
            >
              {updating ?? `${update.version} available`}
            </button>
          )}
          <span className="pointer-events-none hidden text-[11px] text-muted-foreground lg:block">
            {state ? `v${state.version}` : "…"}
          </span>
          <WindowControls maximized={maximized} />
        </div>
      </header>

      {state && !state.accessibilityOk && (
        <div className="flex items-center justify-between gap-3 border-b border-amber-500/30 bg-amber-500/10 px-6 py-2 text-[11px] text-amber-300">
          <span>
            Quad needs Accessibility permission to move windows. Enable Quad under System Settings →
            Privacy &amp; Security → Accessibility, then relaunch.
          </span>
          <button
            type="button"
            onClick={() => api.requestAccessibility()}
            className="shrink-0 rounded-md border border-amber-400/40 px-2 py-1 font-medium text-amber-200 transition-colors hover:bg-amber-500/20"
          >
            Open Settings
          </button>
        </div>
      )}

      <main className="relative flex-1 overflow-y-auto">
        {!state ? (
          <Loading />
        ) : tab === "guide" ? (
          <Guide state={state} notify={notify} />
        ) : tab === "settings" ? (
          <Settings state={state} setState={setState} notify={notify} />
        ) : (
          <About state={state} notify={notify} />
        )}
      </main>

      <Toast toast={toast} />
      <ResizeBorders disabled={maximized} />
    </div>
  );
}
