import { RefreshCw } from "lucide-react";
import { Button } from "./ui/button";
import { EmptyState } from "./AsyncState";
import { pageFailure } from "./pageFailureCopy";

export function PageFailure({ error }: { error: unknown }) {
  const copy = pageFailure(error);
  return (
    <section role="alert" aria-label="Page unavailable">
      <EmptyState
        icon={RefreshCw}
        title={copy.title}
        description={copy.description}
        action={
          <Button onClick={() => window.location.reload()}>Reload page</Button>
        }
      />
    </section>
  );
}
