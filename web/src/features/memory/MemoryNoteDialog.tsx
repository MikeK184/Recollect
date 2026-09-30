import { useState } from "react";
import {
  Alert,
  Button,
  Group,
  Modal,
  Stack,
  Text,
  Textarea,
} from "@mantine/core";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import { ErrorState } from "../../components/AsyncState";
import { useIdempotency } from "../../useIdempotency";

export function MemoryNoteDialog({
  brain,
  close,
  structured,
}: {
  brain: Brain;
  close: () => void;
  structured?: () => void;
}) {
  const [text, setText] = useState("");
  const command = useIdempotency();
  const cache = useQueryClient();
  const settings = useQuery({
    queryKey: ["automation", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
        }),
      ),
  });
  const save = useMutation({
    mutationFn: async () => {
      const body = {
        title: text.trim().split("\n")[0].slice(0, 120),
        media_type: "text/plain",
        content: text.trim(),
        retain_content: true,
        retention_class: "document",
        group_ids: [],
      };
      return result(
        await client.POST("/api/brains/{brain}/sources", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
      close();
    },
  });
  const policy = !settings.error && settings.data?.models.current.policy;
  return (
    <Modal
      opened
      onClose={() => !save.isPending && close()}
      title="Add a memory"
      size="lg"
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (text.trim()) save.mutate();
        }}
      >
        <Stack>
          <Text size="sm">
            Write what is worth remembering. Recollect keeps your note as
            evidence and learns from it automatically when autonomous memory is
            enabled.
          </Text>
          <Textarea
            label="What should this Brain remember?"
            placeholder="A useful finding, decision or lesson…"
            value={text}
            onChange={(e) => setText(e.currentTarget.value)}
            minRows={6}
            autosize
            maxLength={16000}
            required
            data-autofocus
          />
          {policy &&
            (!policy.enabled ||
              !policy.autonomous_memory ||
              !policy.content_classes.includes("document")) && (
              <Alert color="gray">
                This Brain’s current policy stores the note as evidence. Enable
                autonomous memory under Settings → AI &amp; automation for
                automatic learning.
              </Alert>
            )}
          <Text size="xs" c="dimmed">
            Saved in this Brain. Your agent attaches task and repository scope
            to contributions made through the plugin.
          </Text>
          <ErrorState error={save.error ?? settings.error} />
          <Group justify={structured ? "space-between" : "flex-end"}>
            {structured && (
              <Button
                variant="subtle"
                onClick={structured}
                disabled={save.isPending}
              >
                Structured entry
              </Button>
            )}
            <Button
              type="submit"
              loading={save.isPending}
              disabled={!text.trim()}
            >
              Save note
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}
