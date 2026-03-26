import type { ReactNode } from "react";

import { ItemLabel } from "@/components/ItemLabel";

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
        <ItemLabel primary={label} secondary={meta} />
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
