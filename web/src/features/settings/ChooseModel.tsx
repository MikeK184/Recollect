import { useEffect, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { client, result, type Brain } from "../../api";
import type { components } from "../../api-schema";
import { ErrorState } from "../../components/AsyncState";
import { useIdempotency } from "../../useIdempotency";
import { Alert, Button, Modal, Select, Spinner } from "../../components/ui";

type Automation = components["schemas"]["AutomationSettings"];

/** A Brain has no text model until its administrator chooses one. */
export const needsModel = (policy: { text_model: string } | undefined) =>
  !!policy && !policy.text_model;

const providerName = (provider: string) =>
  provider === "openrouter" ? "OpenRouter" : "OpenAI";

/**
 * The state of a Brain with no selected text model: capture works, model
 * processing is off, and an administrator can pick a model right here.
 */
export function ChooseModelNotice({ brain }: { brain: Brain }) {
  const [open, setOpen] = useState(false);
  const admin = brain.role === "admin" && !brain.archived;
  return (
    <>
      <Alert
        variant="warning"
        title="Choose a model"
        actions={
          admin ? (
            <Button size="sm" onClick={() => setOpen(true)}>
              Choose a model
            </Button>
          ) : undefined
        }
      >
        <p>
          No text model is selected for this Brain, so nothing is sent to an AI
          provider. Capture, search and your evidence keep working. Choose a
          model to turn on automatic memory, semantic search and Ask.
        </p>
        {!admin && (
          <p className="mt-1 text-caption text-muted-foreground">
            An administrator of this Brain can choose the model.
          </p>
        )}
      </Alert>
      {open && <ChooseModelDialog brain={brain} close={() => setOpen(false)} />}
    </>
  );
}

function ChooseModelDialog({
  brain,
  close,
}: {
  brain: Brain;
  close: () => void;
}) {
  const automation = useQuery({
    queryKey: ["automation", brain.id],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
        }),
      ),
    retry: false,
    gcTime: 0,
  });
  return (
    <Modal opened onClose={close} title="Choose a model" size="md">
      {automation.data ? (
        <ChooseModelForm
          brain={brain}
          automation={automation.data}
          close={close}
        />
      ) : automation.error ? (
        <ErrorState
          error={automation.error}
          retry={() => automation.refetch()}
        />
      ) : (
        <Spinner label="Loading model settings" />
      )}
    </Modal>
  );
}

function ChooseModelForm({
  brain,
  automation,
  close,
}: {
  brain: Brain;
  automation: Automation;
  close: () => void;
}) {
  const provider = automation.models.current.policy.provider;
  const [model, setModel] = useState<string | null>(null);
  const command = useIdempotency();
  const cache = useQueryClient();
  // The server only accepts a model that the account's fresh model list
  // contains; opening the dialog is the explicit action that refreshes it.
  const refresh = useMutation({
    mutationFn: async () =>
      result(
        await client.POST("/api/brains/{brain}/models/catalogue", {
          params: { path: { brain: brain.id }, query: { provider } },
        }),
      ),
    onSuccess: (data) =>
      cache.setQueryData(["model-catalogue", brain.id, provider], data),
  });
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    refresh.mutate();
  }, []);
  const catalogue = useQuery({
    queryKey: ["model-catalogue", brain.id, provider],
    queryFn: async () =>
      result(
        await client.GET("/api/brains/{brain}/models/catalogue", {
          params: { path: { brain: brain.id }, query: { provider } },
        }),
      ),
    retry: false,
  });
  const choices = (catalogue.data?.models ?? []).filter(
    (m) => m.kind === "text" && m.selectable,
  );
  const entry = choices.find((m) => m.id === model);
  const save = useMutation({
    mutationFn: async (text_model: string) => {
      const body = {
        model_change: automation.models.current.change_id,
        capture_change: automation.capture.change_id,
        text_model,
      };
      return result(
        await client.PUT("/api/brains/{brain}/automation", {
          params: { path: { brain: brain.id } },
          body,
          headers: { "Idempotency-Key": command.forInput(body) },
        }),
      );
    },
    onSuccess: async () => {
      command.reset();
      await cache.invalidateQueries({
        predicate: (q) => q.queryKey.includes(brain.id),
      });
      close();
    },
  });
  const listFailed =
    !!catalogue.data?.error_code || refresh.isError || catalogue.isError;
  return (
    <div className="grid gap-4">
      <p className="m-0 text-body text-foreground-secondary">
        Choose the {providerName(provider)} text model for this Brain. This also
        turns on autonomous memory with the managed limits: eligible retained
        sources and questions may be sent to {providerName(provider)} within a
        daily allowance. You can change the model or the permissions later in
        Settings.
      </p>
      {!automation.models.installed.credentials_present && (
        <Alert variant="warning">
          The {providerName(provider)} credential is missing on this
          installation. Model processing cannot run until an administrator adds
          it.
        </Alert>
      )}
      <Select
        label="Text model"
        placeholder={
          refresh.isPending
            ? "Loading available models…"
            : choices.length
              ? "Choose a model"
              : "No models available"
        }
        data={choices.map((m) => ({ value: m.id, label: m.id }))}
        value={model}
        onChange={(value) => {
          save.reset();
          setModel(value);
        }}
        disabled={refresh.isPending || save.isPending}
        allowDeselect={false}
        inputClassName="mono"
      />
      {entry && (
        <p className="-mt-2 m-0 mono text-caption text-muted-foreground">
          {entry.input_usd_per_million == null
            ? "Price unknown"
            : `$${entry.input_usd_per_million.toFixed(2)} input`}
          {entry.output_usd_per_million == null
            ? ""
            : ` · $${entry.output_usd_per_million.toFixed(2)} output`}{" "}
          / 1M tokens · reference rates, checked {entry.checked_on}
        </p>
      )}
      {listFailed && (
        <Alert
          variant="destructive"
          actions={
            <Button
              variant="destructive-ghost"
              size="sm"
              loading={refresh.isPending}
              onClick={() => refresh.mutate()}
            >
              Retry
            </Button>
          }
        >
          The model list could not be loaded. Check the provider credential and
          try again.
        </Alert>
      )}
      <ErrorState error={save.error} />
      <div className="-mx-5 -mb-5 flex justify-end gap-2 border-t border-border bg-muted px-5 py-3">
        <Button variant="secondary" onClick={close} disabled={save.isPending}>
          Cancel
        </Button>
        <Button
          loading={save.isPending}
          disabled={!model}
          onClick={() => model && save.mutate(model)}
        >
          Use this model
        </Button>
      </div>
    </div>
  );
}
