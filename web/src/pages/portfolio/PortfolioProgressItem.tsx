import type { ReactNode } from "react";

type PortfolioProgressItemProps = {
  label: string;
  meta?: string;
  percentage: number;
  value: ReactNode;
};

export function PortfolioProgressItem({
  label,
  meta,
  percentage,
  value,
}: PortfolioProgressItemProps) {
  return (
    <div className="space-y-1">
      <div className="flex items-end justify-between text-sm">
        <div className="flex flex-col">
          <span className="font-medium">{label}</span>
          {meta ? <span className="text-xs text-muted-foreground opacity-80">{meta}</span> : null}
        </div>
        <div className="text-right">{value}</div>
      </div>
      <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className="h-full bg-primary transition-all duration-500"
          style={{ width: `${percentage}%` }}
        />
      </div>
    </div>
  );
}
