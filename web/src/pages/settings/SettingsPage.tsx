import { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import { getConfigApiUrl } from "@/lib/env";

export function SettingsPage() {
  const [markdown, setMarkdown] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    fetch(getConfigApiUrl())
      .then((res) => {
        if (!res.ok) throw new Error("failed");
        return res.json() as Promise<{ markdown: string }>;
      })
      .then((data) => setMarkdown(data.markdown))
      .catch(() => setError(true));
  }, []);

  return (
    <div className="mx-auto w-full max-w-2xl">
      <h1 className="mb-6 text-2xl font-semibold">Settings</h1>
      {error && (
        <p className="text-destructive">Failed to load configuration.</p>
      )}
      {!error && markdown === null && (
        <p className="text-muted-foreground">Loading…</p>
      )}
      {markdown !== null && (
        <article className="prose prose-sm dark:prose-invert max-w-none">
          <ReactMarkdown>{markdown}</ReactMarkdown>
        </article>
      )}
    </div>
  );
}
