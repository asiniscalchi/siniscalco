import { AssistantChat } from "@/components/assistant";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export function AssistantPage() {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight">Assistant</h1>
        <p className="max-w-3xl text-sm text-muted-foreground">
          Assistant chat backed by the portfolio backend. Ask about your
          portfolio, accounts, assets, transactions, or transfers.
        </p>
      </div>

      <Card className="flex min-h-[42rem] flex-col">
        <CardHeader>
          <CardTitle>Assistant Workspace</CardTitle>
          <CardDescription>
            Full-page version of the same assistant chat used by the header popup.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex min-h-0 flex-1 flex-col p-0">
          <AssistantChat className="flex-1" />
        </CardContent>
      </Card>
    </div>
  );
}
