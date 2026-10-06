import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Button,
  Group,
  Drawer,
  FileButton,
  Stack,
  Text,
  Textarea,
  TextInput,
} from "@mantine/core";
import { useMutation } from "@tanstack/react-query";
import { client, result, type Brain } from "../api";
import { useIdempotency } from "../useIdempotency";
import { ErrorState as Failure } from "./AsyncState";
import { BrainIcon, prepareBrainIcon } from "./BrainIcon";
import "../features/workspace/control-panel.css";

export function BrainForm({
  opened,
  close,
  saved,
  brain,
  inline = false,
  disabled = false,
}: {
  opened: boolean;
  close: () => void;
  saved: (brain: Brain) => void;
  brain?: Brain;
  inline?: boolean;
  disabled?: boolean;
}) {
  const command = useIdempotency();
  const [name, setName] = useState(brain?.name ?? "");
  const [description, setDescription] = useState(brain?.description ?? "");
  const [icon, setIcon] = useState<Awaited<
    ReturnType<typeof prepareBrainIcon>
  > | null>(null);
  const [removeIcon, setRemoveIcon] = useState(false);
  const [preparing, setPreparing] = useState(false);
  const [iconError, setIconError] = useState<Error | null>(null);
  const [persisted, setPersisted] = useState<Brain | null>(null);
  const sequence = useRef(0);
  useEffect(() => {
    if (opened) {
      setName(brain?.name ?? "");
      setDescription(brain?.description ?? "");
    }
  }, [opened, brain?.id]);
  const reset = () => {
    sequence.current++;
    setPreparing(false);
    setIcon(null);
    setRemoveIcon(false);
    setIconError(null);
    setPersisted(null);
    save.reset();
    command.reset();
    setName(brain?.name ?? "");
    setDescription(brain?.description ?? "");
  };
  const finish = () => {
    if (save.isPending) return;
    if (persisted) saved(persisted);
    else close();
    reset();
  };
  const save = useMutation({
    mutationFn: async () => {
      if (disabled)
        throw new Error(
          "Brain editing is no longer available. Reload settings.",
        );
      const data =
        persisted ??
        (brain
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
            ));
      // Retain the successful metadata command across icon failures. A retry
      // must never create a second Brain, even after a lost upload response.
      setPersisted(data);
      if (icon)
        return result(
          await client.PUT("/api/brains/{id}/icon", {
            params: { path: { id: data.id } },
            body: "",
            bodySerializer: () => icon.blob,
            headers: { "Content-Type": "image/png" },
          }),
        );
      if (removeIcon && data.icon_revision)
        return result(
          await client.DELETE("/api/brains/{id}/icon", {
            params: { path: { id: data.id } },
          }),
        );
      return data;
    },
    onSuccess: (data) => {
      saved(data);
      reset();
    },
  });
  const upload = async (file: File | null) => {
    if (!file) return;
    const token = ++sequence.current;
    setPreparing(true);
    setIconError(null);
    try {
      const ready = await prepareBrainIcon(file);
      if (token === sequence.current) {
        setIcon(ready);
        setRemoveIcon(false);
      }
    } catch (error) {
      if (token === sequence.current)
        setIconError(
          error instanceof Error
            ? error
            : new Error("The icon could not be read."),
        );
    } finally {
      if (token === sequence.current) setPreparing(false);
    }
  };
  const form = (
    <form
      className={inline ? "brain-inline-form" : undefined}
      aria-label={inline ? "Edit Brain" : undefined}
      role={inline ? "region" : undefined}
      onSubmit={(event) => {
        event.preventDefault();
        if (
          !disabled &&
          !save.isPending &&
          name.trim() &&
          !preparing &&
          !iconError
        )
          save.mutate();
      }}
    >
      <Stack>
        <Group className={inline ? "brain-inline-icon" : undefined}>
          {icon ? (
            <span className="brain-artwork" style={{ width: 56, height: 56 }}>
              <img src={icon.preview} alt="Brain icon preview" />
            </span>
          ) : (
            <BrainIcon
              id={brain?.id ?? "draft"}
              revision={removeIcon ? null : brain?.icon_revision}
              size={56}
            />
          )}
          <Stack gap={4}>
            <Group gap="xs">
              <FileButton
                onChange={(file) => void upload(file)}
                accept="image/png,image/svg+xml,image/x-icon,image/vnd.microsoft.icon,.ico"
              >
                {(props) => (
                  <Button
                    {...props}
                    variant="default"
                    size="xs"
                    loading={preparing}
                    disabled={disabled || save.isPending}
                  >
                    Upload icon
                  </Button>
                )}
              </FileButton>
              {(icon || (brain?.icon_revision && !removeIcon)) && (
                <Button
                  size="xs"
                  variant="subtle"
                  disabled={disabled || save.isPending || preparing}
                  onClick={() => {
                    setIcon(null);
                    setRemoveIcon(true);
                    setIconError(null);
                  }}
                >
                  Remove icon
                </Button>
              )}
            </Group>
            <Text size="xs" c="dimmed">
              PNG, SVG or ICO · up to 512 KiB
            </Text>
          </Stack>
        </Group>
        <Failure error={iconError} />
        <TextInput
          label="Name"
          placeholder="e.g. Platform engineering"
          required
          disabled={disabled || !!persisted || save.isPending}
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
          disabled={disabled || !!persisted || save.isPending}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
        {persisted && save.isError && (
          <Text size="sm">
            Brain saved. Retry the icon or close to keep the default.
          </Text>
        )}
        <Failure error={save.error} />
        {!brain && (
          <div className="creation-preview">
            {icon ? (
              <span className="brain-artwork">
                <img src={icon.preview} alt="" />
              </span>
            ) : (
              <BrainIcon id="draft" />
            )}
            <div>
              <strong>{name.trim() || "Your new Brain"}</strong>
              <p>{description.trim() || "A space for what you know."}</p>
            </div>
          </div>
        )}
        {!brain && (
          <Alert color="brand" title="Autonomous memory included">
            Supported agent capture, learning, semantic search and Ask use
            managed defaults. This Brain’s retained evidence and questions may
            be processed by the installation’s model provider. Connect your
            agent once; Recollect handles the memory lifecycle.
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={finish} disabled={save.isPending}>
            {persisted ? "Close" : "Cancel"}
          </Button>
          <Button
            type="submit"
            disabled={disabled || !name.trim() || preparing || !!iconError}
            loading={save.isPending}
          >
            {persisted ? "Retry icon" : brain ? "Save changes" : "Create Brain"}
          </Button>
        </Group>
      </Stack>
    </form>
  );
  if (inline) return opened ? form : null;
  return (
    <Drawer
      opened={opened}
      onClose={finish}
      title={brain ? "Edit Brain" : "Create a Brain"}
      position="right"
      size="lg"
      className="control-drawer"
    >
      {form}
    </Drawer>
  );
}
