import { useState } from "react";
import { ListTree } from "lucide-react";
import { Alert, Button, Modal, TextareaField } from "../../components/ui";
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
      description="Write what is worth remembering. Recollect keeps your note as evidence and learns from it automatically when autonomous memory is enabled."
      size="lg"
      footer={
        <div className="flex w-full items-center gap-2">
          {structured && (
            <Button
              variant="ghost"
              onClick={structured}
              disabled={save.isPending}
            >
              <ListTree aria-hidden="true" />
              Structured entry
            </Button>
          )}
          <Button
            type="submit"
            form="memory-note"
            className="ml-auto"
            loading={save.isPending}
            disabled={!text.trim()}
          >
            Save note
          </Button>
        </div>
      }
    >
      <form
        id="memory-note"
        className="grid gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          if (text.trim()) save.mutate();
        }}
      >
        <TextareaField
          label="What should this Brain remember?"
          placeholder="A useful finding, decision or lesson…"
          value={text}
          onChange={(e) => setText(e.currentTarget.value)}
          minRows={6}
          autosize
          maxLength={16000}
          required
        />
        {policy &&
          (!policy.enabled ||
            !policy.autonomous_memory ||
            !policy.content_classes.includes("document")) && (
            <Alert variant="default" role="status">
              This Brain’s current policy stores the note as evidence. Enable
              autonomous memory under Settings → AI &amp; automation for
              automatic learning.
            </Alert>
          )}
        <p className="m-0 text-caption text-muted-foreground">
          Saved in this Brain. Your agent attaches task and repository scope to
          contributions made through the plugin.
        </p>
        <ErrorState error={save.error ?? settings.error} />
      </form>
    </Modal>
  );
}
