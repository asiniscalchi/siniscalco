import { TopMoversCard } from "./TopMoversCard";
import { AssetsTableCard } from "./AssetsTableCard";

export function AssetsPage() {
  return (
    <div className="mx-auto flex min-w-0 w-full max-w-4xl flex-col gap-6 overflow-x-hidden">
      <TopMoversCard />
      <AssetsTableCard />
    </div>
  );
}
