import { cn } from "../lib/utils";
import { api } from "../lib/bridge";

// Custom min / maximize-restore / close buttons (Windows-style ordering, right-aligned).
export function WindowControls({ maximized }: { maximized: boolean }) {
  return (
    <div data-no-drag className="relative z-[70] flex items-center">
      <CtrlButton label="Minimize" onClick={() => api.windowMinimize()}>
        <svg width="12" height="12" viewBox="0 0 12 12">
          <rect x="2" y="5.5" width="8" height="1" fill="currentColor" />
        </svg>
      </CtrlButton>

      <CtrlButton label={maximized ? "Restore" : "Maximize"} onClick={() => api.windowToggleMaximize()}>
        {maximized ? (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1">
            <rect x="3.5" y="1.5" width="6" height="6" />
            <rect x="1.5" y="3.5" width="6" height="6" fill="oklch(var(--card))" />
          </svg>
        ) : (
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1">
            <rect x="2" y="2" width="8" height="8" />
          </svg>
        )}
      </CtrlButton>

      <CtrlButton label="Close" danger onClick={() => api.windowClose()}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2">
          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" />
        </svg>
      </CtrlButton>
    </div>
  );
}

function CtrlButton({
  children,
  onClick,
  label,
  danger,
}: {
  children: React.ReactNode;
  onClick: () => void;
  label: string;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex h-9 w-12 items-center justify-center text-muted-foreground transition-colors",
        danger ? "hover:bg-destructive hover:text-destructive-foreground" : "hover:bg-muted hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
