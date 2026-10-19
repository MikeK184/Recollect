import { Badge, Button, Card, Group, Stack, Text } from "@mantine/core";
import { BookMarked } from "lucide-react";
import { EmptyState } from "./components/AsyncState";
import { SkeletonRows } from "./components/Skeleton";
import { staggerStyle } from "./components/Motion";
import { CompositeStatus, Pages, kindBadgeColor, label } from "./claimsShared";
import { memoryKinds } from "./MemoryForms";
import type { ClaimsRouteState } from "./useClaimsRoute";

// The loading skeleton, empty state, record rows and pager for the memory list.
export function ClaimsList({ state }: { state: ClaimsRouteState }) {
  const {
    claims,
    data,
    query,
    filtered,
    stable,
    setSelected,
    offset,
    setOffset,
  } = state;
  return (
    <>
      {claims.isPending && <SkeletonRows label="Loading memory…" />}
      {data && (
        <>
          {data.items.length === 0 && !data.unavailable?.length && (
            <EmptyState
              icon={BookMarked}
              title={
                query || filtered
                  ? "No matching memory"
                  : "Your knowledge will gather here"
              }
              description={
                query || filtered
                  ? "Try another phrase or adjust the filters. Historical and qualified records can be inspected separately."
                  : "Recollect learns from permitted sources and agent activity. You can also add an evidence-backed memory yourself."
              }
            />
          )}
          {!!data.items.length && (
            <div className="feature-list">
              {stable.pending > 0 && (
                <Button
                  variant="light"
                  size="xs"
                  m="sm"
                  onClick={stable.reveal}
                >
                  {stable.pending} new records · Show updates
                </Button>
              )}
              {stable.rows.map((view, index) => (
                <article
                  className="memory-record rc-enter"
                  style={staggerStyle(index)}
                  key={view.revision.claim_id}
                >
                  {/* Dense row: one primary identifier line plus one composite
                      status line. Value, evidence and knowledge time move to
                      the shared lineage inspector. */}
                  <Stack gap="xs">
                    <Group justify="space-between" align="start" gap="xs">
                      <Button
                        variant="subtle"
                        justify="start"
                        px={0}
                        h="auto"
                        className="memory-record-title"
                        onClick={() => setSelected(view.revision.claim_id)}
                      >
                        <span className="memory-assertion">
                          <span className="memory-subject">
                            {view.revision.content.subject} ·{" "}
                            {label(view.revision.content.predicate)}
                          </span>
                          <strong>{view.revision.content.value}</strong>
                        </span>
                      </Button>
                      <Badge
                        variant="light"
                        color={
                          kindBadgeColor[view.revision.content.kind] ?? "gray"
                        }
                      >
                        {memoryKinds.find(
                          (item) => item.value === view.revision.content.kind,
                        )?.label ?? label(view.revision.content.kind)}
                      </Badge>
                    </Group>
                    <Group gap="md" className="memory-row-meta">
                      <Text size="xs" c="dimmed">
                        {view.revision.actor_name} ·{" "}
                        {new Date(
                          view.revision.recorded_at,
                        ).toLocaleDateString()}
                      </Text>
                      <CompositeStatus view={view} />
                    </Group>
                  </Stack>
                </article>
              ))}
            </div>
          )}
          {data.unavailable?.map((item) => (
            <Card withBorder key={item.claim_id} mt="sm">
              <Button
                variant="subtle"
                onClick={() => setSelected(item.claim_id)}
              >
                Unavailable memory · {item.claim_id.slice(0, 8)}
              </Button>
              <Text size="sm" c="dimmed">
                Content is unavailable. Inspect its remaining history and erased
                intervals.
              </Text>
            </Card>
          ))}
          <Pages
            offset={offset}
            total={data.total_candidates}
            nextOffset={data.next_offset}
            onChange={setOffset}
          />
        </>
      )}
    </>
  );
}
