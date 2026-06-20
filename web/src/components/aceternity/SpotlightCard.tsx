import { useRef, useState, type ReactNode, type MouseEvent } from "react";
import { cn } from "../../lib/utils";

// Aceternity UI — Card Spotlight (radial highlight that follows the cursor).
export function SpotlightCard({
  children,
  className,
  color = "oklch(var(--accent) / 0.16)",
  onClick,
}: {
  children: ReactNode;
  className?: string;
  color?: string;
  onClick?: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const [hovered, setHovered] = useState(false);

  function onMove(e: MouseEvent<HTMLDivElement>) {
    const el = ref.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    setPos({ x: e.clientX - rect.left, y: e.clientY - rect.top });
  }

  return (
    <div
      ref={ref}
      onMouseMove={onMove}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
      className={cn(
        "group relative overflow-hidden rounded-xl border border-border bg-card/50 transition-colors duration-200 hover:border-foreground/20",
        onClick && "cursor-pointer",
        className,
      )}
    >
      <div
        className="pointer-events-none absolute -inset-px opacity-0 transition-opacity duration-300 group-hover:opacity-100"
        style={{
          opacity: hovered ? 1 : 0,
          background: `radial-gradient(220px circle at ${pos.x}px ${pos.y}px, ${color}, transparent 70%)`,
        }}
      />
      <div className="relative z-10 h-full">{children}</div>
    </div>
  );
}
