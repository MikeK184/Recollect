import { useState } from "react";
import {
  Badge,
  Button,
  Card,
  Divider,
  Group,
  Select,
  Text,
  TextInput,
  Title,
} from "@mantine/core";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { ArrowRight, Boxes, Layers3, Plus, Search, Shield } from "lucide-react";
import { client, result } from "../../api";
import { useWorkspace } from "../../app/context";
import { BrainForm } from "../../components/BrainForm";
import { PageHeader } from "../../components/PageHeader";
import {
  EmptyState,
  ErrorState,
  LoadingState,
} from "../../components/AsyncState";

export function BrainsPage() {
  const user = useWorkspace();
  const navigate = useNavigate();
  const cache = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [search, setSearch] = useState("");
  const [view, setView] = useState("active");
  const query = useQuery({
    queryKey: ["brains"],
    queryFn: async ({ signal }) =>
      result(await client.GET("/api/brains", { signal })),
  });
  const brains = query.isError ? [] : (query.data ?? []);
  const visible = brains.filter((b) => {
    const matches = `${b.name} ${b.description}`
      .toLocaleLowerCase()
      .includes(search.toLocaleLowerCase());
    return (
      matches &&
      (view === "archived" ? b.archived : !b.archived) &&
      (view === "mine"
        ? b.owner_id === user.user.id
        : view === "shared"
          ? b.owner_id !== user.user.id
          : true)
    );
  });
  return (
    <>
      <PageHeader
        title="Your Brains"
        eyebrow="A place for what you know"
        description="Knowledge, decisions, and the evidence that connects them."
        actions={
          <Button
            leftSection={<Plus size={18} />}
            onClick={() => setCreating(true)}
          >
            Create Brain
          </Button>
        }
      />
      <div className="filter-bar brain-filter-bar">
        <TextInput
          aria-label="Search Brains"
          placeholder="Find a Brain…"
          leftSection={<Search size={16} />}
          value={search}
          onChange={(e) => setSearch(e.currentTarget.value)}
        />
        <Select
          aria-label="Brain view"
          value={view}
          onChange={(v) => setView(v ?? "active")}
          allowDeselect={false}
          data={[
            { value: "active", label: "All active Brains" },
            { value: "mine", label: "Owned by you" },
            { value: "shared", label: "Shared with you" },
            { value: "archived", label: "Archived" },
          ]}
        />
        <Text size="xs" c="dimmed" ml="auto">
          {visible.length} {visible.length === 1 ? "Brain" : "Brains"} · Only
          spaces you can access
        </Text>
      </div>
      <ErrorState error={query.error} retry={() => void query.refetch()} />
      {query.isPending ? (
        <LoadingState label="Loading your Brains…" />
      ) : !query.isError && !visible.length ? (
        <EmptyState
          icon={Boxes}
          title={
            brains.length
              ? "No Brains in this view"
              : "Room for your first idea"
          }
          description={
            brains.length
              ? "Try another name or view to find your space."
              : "Create a Brain for your personal knowledge, a project, or your team."
          }
          action={
            !brains.length && (
              <Button variant="light" onClick={() => setCreating(true)}>
                Create your first Brain
              </Button>
            )
          }
        />
      ) : (
        <div className="brain-grid">
          {visible.map((brain) => (
            <Link
              to="/brains/$brainId/ask"
              params={{ brainId: brain.id }}
              search={{}}
              key={brain.id}
              className="brain-link"
            >
              <Card withBorder padding="lg" className="brain-card">
                <Group justify="space-between">
                  <div className="brain-symbol">
                    <Layers3 size={24} />
                  </div>
                  <Badge color={brain.archived ? "gray" : "brand"}>
                    {brain.archived ? "Archived" : "Active"}
                  </Badge>
                </Group>
                <Title order={3} mt="lg">
                  {brain.name}
                </Title>
                <Text c="dimmed" size="sm" mt="xs" lineClamp={3} mih={60}>
                  {brain.description || "A space for knowledge and context."}
                </Text>
                <Divider mt="lg" mb="md" />
                <Group justify="space-between">
                  <Group gap={6}>
                    <Shield size={14} />
                    <Text size="xs" c="dimmed">
                      {brain.owner_id === user.user.id ? "Owner" : brain.role}
                    </Text>
                  </Group>
                  <ArrowRight size={18} />
                </Group>
              </Card>
            </Link>
          ))}
        </div>
      )}
      <BrainForm
        opened={creating}
        close={() => setCreating(false)}
        saved={(brain) => {
          setCreating(false);
          void cache.invalidateQueries({ queryKey: ["brains"] });
          void navigate({
            to: "/brains/$brainId/ask",
            params: { brainId: brain.id },
            search: {},
          });
        }}
      />
    </>
  );
}
