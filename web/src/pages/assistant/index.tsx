import { AssistantChatPanel, AssistantRuntimeBoundary, ThreadList } from "@/components/assistant";
import { Card, CardContent } from "@/components/ui/card";

export function AssistantPage() {
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h1 className="text-2xl font-semibold tracking-tight">Assistant</h1>
        <p className="max-w-3xl text-sm text-muted-foreground">
          Assistant chat backed by the portfolio backend. Ask about your portfolio, accounts,
          assets, transactions, or transfers.
        </p>
      </div>

      <AssistantRuntimeBoundary>
        <Card className="flex min-h-[42rem] flex-col overflow-hidden">
          <CardContent className="flex min-h-0 flex-1 flex-col p-0">
            <div className="flex min-h-0 flex-1">
              {/* Thread list sidebar */}
              <div className="hidden w-56 shrink-0 flex-col border-r bg-muted/20 p-3 sm:flex">
                <ThreadList className="flex-1" />
              </div>

              {/* Chat panel */}
              <AssistantChatPanel className="min-h-0 flex-1" />
            </div>
          </CardContent>
        </Card>
      </AssistantRuntimeBoundary>
    </div>
  );
}
