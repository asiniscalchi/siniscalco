import { Card, CardHeader } from "@/components/ui/card";

import { PortfolioPageHeader } from "./PortfolioPageHeader";

export function PortfolioLoadingState() {
  return (
    <div className="flex flex-col gap-4">
      <PortfolioPageHeader />
      <div className="grid gap-4 md:grid-cols-2">
        {Array.from({ length: 4 }).map((_, index) => (
          <Card key={index} className="border-dashed bg-background/70">
            <CardHeader>
              <div className="h-5 w-32 rounded-full bg-muted" />
              <div className="h-4 w-48 rounded-full bg-muted" />
            </CardHeader>
          </Card>
        ))}
      </div>
    </div>
  );
}
