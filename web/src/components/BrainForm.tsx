import { useState } from "react";
import {
  Alert,
  Button,
  Group,
  Modal,
  Stack,
  Text,
  Textarea,
  TextInput,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { client, result, type Brain } from "../api";
import { useIdempotency } from "../useIdempotency";
import { ErrorState as Failure } from "./AsyncState";

export function BrainForm({
  opened,
  close,
  saved,
  brain,
}: {
  opened: boolean;
  close: () => void;
  saved: (brain: Brain) => void;
  brain?: Brain;
}) {
  const command = useIdempotency();
  const [name, setName] = useState(brain?.name ?? "");
  const [description, setDescription] = useState(brain?.description ?? "");
  const save = useMutation({
    mutationFn: async () =>
      brain
        ? result(
            await client.PATCH("/api/brains/{id}", {
              params: { path: { id: brain.id } },
              body: { name, description },
              headers: {
                "Idempotency-Key": command.forInput({ name, description }),
              },
            }),
          )
        : result(
            await client.POST("/api/brains", {
              body: { name, description, managed_memory: true },
              headers: {
                "Idempotency-Key": command.forInput({ name, description }),
              },
            }),
          ),
    onSuccess: (data) => {
      command.reset();
      saved(data);
      if (!brain) {
        setName("");
        setDescription("");
      }
    },
  });
  return (
    <Modal
      opened={opened}
      onClose={close}
      title={brain ? "Edit Brain" : "Create a Brain"}
      centered
    >
      <form
        onSubmit={(event) => {
          event.preventDefault();
          save.mutate();
        }}
      >
        <Stack>
          <Text c="dimmed" size="sm">
            Choose a name that makes this space easy to recognize.
          </Text>
          <TextInput
            label="Name"
            placeholder="e.g. Platform engineering"
            required
            maxLength={120}
            value={name}
            onChange={(event) => setName(event.target.value)}
            data-autofocus
          />
          <Textarea
            label="Description"
            placeholder="What belongs in this Brain?"
            maxLength={2000}
            minRows={3}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
          <Failure error={save.error} />
          {!brain && (
            <Alert color="brand" title="Autonomous memory included">
              Supported agent capture, learning, semantic search and Ask use
              managed defaults. This Brain’s retained evidence and questions may
              be processed by the installation’s model provider. Connect your
              agent once; Recollect handles the memory lifecycle.
            </Alert>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={close}>
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={!name.trim()}
              loading={save.isPending}
            >
              {brain ? "Save changes" : "Create Brain"}
            </Button>
          </Group>
        </Stack>
      </form>
    </Modal>
  );
}
