import { useEffect, useId, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { ImageUp, Sparkles, Trash2 } from "lucide-react";
import { client, result, type Brain } from "../api";
import { useIdempotency } from "../useIdempotency";
import { ErrorState as Failure } from "./AsyncState";
import { BrainIcon, prepareBrainIcon } from "./BrainIcon";
import {
  Alert,
  Button,
  Drawer,
  FileButton,
  Input,
  Textarea,
  Field,
} from "./ui";
import { cn } from "../lib/utils";

/**
 * Create or edit a Brain's name, description and icon. Opens as a side sheet
 * ("Create a Brain" / "Edit Brain"), or renders inline as the "Edit Brain"
 * region inside Settings.
 */
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
  const formId = `brain-form-${useId()}`;
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
  const nameInput = useRef<HTMLInputElement>(null);
  // Focus the name once the sheet has mounted, so the sheet still returns
  // focus to the control that opened it.
  useEffect(() => {
    if (!opened || inline) return;
    const timer = setTimeout(() => nameInput.current?.focus(), 0);
    return () => clearTimeout(timer);
  }, [opened, inline]);
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
  const locked = disabled || !!persisted || save.isPending;
  const actions = (
    <>
      <Button variant="secondary" onClick={finish} disabled={save.isPending}>
        {persisted ? "Close" : "Cancel"}
      </Button>
      <Button
        type="submit"
        form={formId}
        disabled={disabled || !name.trim() || preparing || !!iconError}
        loading={save.isPending}
      >
        {persisted ? "Retry icon" : brain ? "Save changes" : "Create Brain"}
      </Button>
    </>
  );
  const form = (
    <form
      id={formId}
      className={cn("flex flex-col gap-4", inline && "max-w-[560px]")}
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
      <div className="flex items-center gap-4">
        {icon ? (
          <span className="grid size-14 shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-muted">
            <img
              src={icon.preview}
              alt="Brain icon preview"
              className="size-full object-cover"
            />
          </span>
        ) : (
          <BrainIcon
            id={brain?.id ?? "draft"}
            revision={removeIcon ? null : brain?.icon_revision}
            size={56}
          />
        )}
        <div className="flex min-w-0 flex-col gap-1.5">
          <div className="flex flex-wrap items-center gap-2">
            <FileButton
              onChange={(file) => void upload(file)}
              accept="image/png,image/svg+xml,image/x-icon,image/vnd.microsoft.icon,.ico"
            >
              {(props) => (
                <Button
                  {...props}
                  variant="secondary"
                  size="sm"
                  loading={preparing}
                  disabled={disabled || save.isPending}
                >
                  {!preparing && <ImageUp aria-hidden="true" />}
                  Upload icon
                </Button>
              )}
            </FileButton>
            {(icon || (brain?.icon_revision && !removeIcon)) && (
              <Button
                size="sm"
                variant="ghost"
                disabled={disabled || save.isPending || preparing}
                onClick={() => {
                  setIcon(null);
                  setRemoveIcon(true);
                  setIconError(null);
                }}
              >
                <Trash2 aria-hidden="true" />
                Remove icon
              </Button>
            )}
          </div>
          <p className="m-0 text-caption text-muted-foreground">
            PNG, SVG or ICO · up to <span className="mono">512 KiB</span>
          </p>
        </div>
      </div>
      <Failure error={iconError} />
      <Field label="Name" required>
        <Input
          placeholder="e.g. Platform engineering"
          disabled={locked}
          maxLength={120}
          value={name}
          onChange={(event) => setName(event.target.value)}
          ref={nameInput}
        />
      </Field>
      <Field label="Description">
        <Textarea
          placeholder="What belongs in this Brain?"
          maxLength={2000}
          minRows={3}
          disabled={locked}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
        />
      </Field>
      {persisted && save.isError && (
        <p className="m-0 text-body text-foreground-secondary">
          Brain saved. Retry the icon or close to keep the default.
        </p>
      )}
      <Failure error={save.error} />
      {!brain && (
        <div className="flex items-center gap-3 rounded-lg border border-dashed border-border-strong bg-muted/50 px-3.5 py-3">
          {icon ? (
            <span className="grid size-9 shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-muted">
              <img
                src={icon.preview}
                alt=""
                className="size-full object-cover"
              />
            </span>
          ) : (
            <BrainIcon id="draft" size={36} />
          )}
          <div className="min-w-0">
            <strong className="block truncate text-body font-semibold text-foreground">
              {name.trim() || "Your new Brain"}
            </strong>
            <p className="m-0 line-clamp-2 text-label text-muted-foreground">
              {description.trim() || "A space for what you know."}
            </p>
          </div>
        </div>
      )}
      {!brain && (
        <Alert
          variant="brand"
          role="status"
          icon={<Sparkles aria-hidden="true" />}
          title="Autonomous memory included"
        >
          Supported agent capture, learning, semantic search and Ask use managed
          defaults. Capture starts at once; learning, semantic search and Ask
          run after a model is chosen for this Brain, unless the installation
          names one. This Brain’s retained evidence and questions may then be
          processed by that model’s provider. Connect your agent once; Recollect
          handles the memory lifecycle.
        </Alert>
      )}
      {inline && <div className="flex justify-end gap-2 pt-1">{actions}</div>}
    </form>
  );
  if (inline) return opened ? form : null;
  return (
    <Drawer
      opened={opened}
      onClose={finish}
      title={brain ? "Edit Brain" : "Create a Brain"}
      description={
        brain
          ? "Rename this Brain, describe it or change its icon."
          : "A space for personal knowledge, a project or your team."
      }
      size={480}
      footer={actions}
    >
      {form}
    </Drawer>
  );
}
