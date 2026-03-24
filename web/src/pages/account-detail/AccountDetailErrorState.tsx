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

type AccountDetailErrorStateProps = {
  message: string;
  onRetry: () => void;
};

export function AccountDetailErrorState({
  message,
  onRetry,
}: AccountDetailErrorStateProps) {
  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6">
      <header className="flex items-start justify-between gap-4 rounded-2xl border bg-background p-6 shadow-sm">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.22em] text-muted-foreground">
            Cash Accounts
          </p>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight">
            Account Detail
          </h1>
        </div>
        <Link
          className={cn(buttonVariants({ variant: "outline" }))}
          to="/accounts"
        >
          Back to accounts
        </Link>
      </header>
      <Card className="border-destructive/30 bg-background">
        <CardHeader>
          <CardTitle>Could not load account</CardTitle>
          <CardDescription>{message}</CardDescription>
        </CardHeader>
        <CardFooter className="justify-end gap-3">
          <Link
            className={cn(buttonVariants({ variant: "outline" }))}
            to="/accounts"
          >
            Back to accounts
          </Link>
          <Button onClick={onRetry} type="button">
            Retry
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
