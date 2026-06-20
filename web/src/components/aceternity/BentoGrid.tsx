import type { ReactNode } from "react";
import { cn } from "../../lib/utils";

// Aceternity UI — Bento Grid.
export const BentoGrid = ({ className, children }: { className?: string; children?: ReactNode }) => {
  return (
    <div className={cn("grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4", className)}>
      {children}
    </div>
  );
};

export const BentoGridItem = ({
  className,
  title,
  description,
  header,
  footer,
  onClick,
}: {
  className?: string;
  title?: ReactNode;
  description?: ReactNode;
  header?: ReactNode;
  footer?: ReactNode;
  onClick?: () => void;
}) => {
  return (
    <div
      onClick={onClick}
      className={cn(
        "group/bento row-span-1 flex flex-col justify-between space-y-3 rounded-xl border border-border bg-card/50 p-4 transition-all duration-200 hover:border-foreground/20 hover:bg-card hover:shadow-[0_0_30px_-12px_oklch(var(--accent)/0.6)]",
        onClick && "cursor-pointer",
        className,
      )}
    >
      {header}
      <div className="transition-transform duration-200 group-hover/bento:translate-x-0.5">
        <div className="flex items-center justify-between gap-2">
          <span className="text-sm font-semibold text-foreground">{title}</span>
          {footer}
        </div>
        {description && <p className="mt-1 text-xs text-muted-foreground">{description}</p>}
      </div>
    </div>
  );
};
