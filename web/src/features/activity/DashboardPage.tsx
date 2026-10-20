import { Link } from "@tanstack/react-router";
import { History, MessageSquare } from "lucide-react";
import { useBrain } from "../../app/context";
import { Page } from "../../components/Page";
import { PageHeader } from "../../components/PageHeader";
import { Badge, Button } from "../../components/ui";
import { BrainPipeline } from "./PipelineView";

const roleLabel = (role: string) =>
  `${role.charAt(0).toUpperCase()}${role.slice(1).replaceAll("_", " ")} access`;

export function DashboardPage() {
  const brain = useBrain();
  return (
    <Page>
      <PageHeader
        title="Dashboard"
        description="What this Brain learned recently and what it is working on now."
        meta={
          <>
            <Badge variant="outline">Brain</Badge>
            <Badge>{roleLabel(brain.role)}</Badge>
            {brain.archived && <Badge variant="warning">Archived</Badge>}
          </>
        }
        actions={
          <>
            <Button asChild variant="secondary">
              <Link
                to="/brains/$brainId/activity"
                params={{ brainId: brain.id }}
                search={{ tab: "attention" }}
              >
                <History aria-hidden="true" />
                Activity details
              </Link>
            </Button>
            <Button asChild>
              <Link
                to="/brains/$brainId/ask"
                params={{ brainId: brain.id }}
                search={{}}
              >
                <MessageSquare aria-hidden="true" />
                Ask this Brain
              </Link>
            </Button>
          </>
        }
      />
      <BrainPipeline brain={brain} dashboard />
    </Page>
  );
}
