import { Link } from "react-router-dom";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { buttonVariants } from "@/components/ui/button-variants";
import { cn } from "@/lib/utils";

export function PortfolioErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex flex-col gap-4">
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Could not load portfolio</CardTitle>
          <CardDescription>
            The portfolio overview request failed. Try again to reload the page
            data.
          </CardDescription>
        </CardHeader>
        <CardFooter className="justify-end gap-3">
          <Link className={cn(buttonVariants({ variant: "outline" }))} to="/accounts">
            View accounts
          </Link>
          <Button onClick={onRetry} type="button">
            Retry
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
