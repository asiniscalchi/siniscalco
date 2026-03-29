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
          Basic assistant-ui scaffold for the portfolio app. It currently uses an
          in-memory runtime with canned responses, so the route is ready before a
          real backend chat adapter exists.
        </p>
      </div>

      <Card className="min-h-[42rem]">
        <CardHeader>
          <CardTitle>Assistant Workspace</CardTitle>
          <CardDescription>
            Full-page version of the same assistant chat used by the header popup.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex min-h-0 flex-1 flex-col p-0">
          <AssistantChat className="min-h-[34rem]" />
        </CardContent>
      </Card>
    </div>
  );
}
