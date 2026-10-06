import { Button, Group } from "@mantine/core";
import { Link } from "@tanstack/react-router";
import { History, MessageSquare } from "lucide-react";
import { useBrain } from "../../app/context";
import { PageHeader } from "../../components/PageHeader";
import { BrainPipeline } from "./PipelineView";

export function DashboardPage() {
  const brain = useBrain();
  return (
    <section className="brain-dashboard">
      <PageHeader
        title="Dashboard"
        actions={
          <Group gap="sm">
            <Button
              renderRoot={(props) => (
                <Link
                  {...props}
                  to="/brains/$brainId/activity"
                  params={{ brainId: brain.id }}
                  search={{ tab: "attention" }}
                />
              )}
              variant="default"
              leftSection={<History size={16} />}
            >
              Activity details
            </Button>
            <Button
              renderRoot={(props) => (
                <Link
                  {...props}
                  to="/brains/$brainId/ask"
                  params={{ brainId: brain.id }}
                  search={{}}
                />
              )}
              leftSection={<MessageSquare size={16} />}
            >
              Ask this Brain
            </Button>
          </Group>
        }
      />
      <BrainPipeline brain={brain} dashboard />
    </section>
  );
}
