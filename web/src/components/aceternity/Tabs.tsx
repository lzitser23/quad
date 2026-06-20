import { motion } from "framer-motion";
import { cn } from "../../lib/utils";

// Aceternity UI — Tabs (animated active pill via shared layoutId), squared + accent-tinted.
export interface TabItem {
  id: string;
  label: string;
}

export function Tabs({
  tabs,
  active,
  onChange,
  className,
}: {
  tabs: TabItem[];
  active: string;
  onChange: (id: string) => void;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-wrap items-center gap-1 rounded-md border border-border bg-card/60 p-1", className)}>
      {tabs.map((tab) => {
        const isActive = tab.id === active;
        return (
          <button
            key={tab.id}
            onClick={() => onChange(tab.id)}
            className={cn(
              "relative rounded-[3px] px-4 py-1.5 text-sm font-medium transition-colors",
              isActive ? "text-accent-foreground" : "text-muted-foreground hover:text-foreground",
            )}
          >
            {isActive && (
              <motion.span
                layoutId="active-tab-pill"
                className="absolute inset-0 z-0 rounded-[3px] bg-accent shadow-[0_0_18px_-6px_oklch(var(--accent)/0.9)]"
                transition={{ type: "spring", bounce: 0.2, duration: 0.5 }}
              />
            )}
            <span className="relative z-10">{tab.label}</span>
          </button>
        );
      })}
    </div>
  );
}
