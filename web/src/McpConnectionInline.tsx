import { createPortal } from "react-dom";
import {
  Alert,
  Button,
  Checkbox,
  Input,
  JsonInput,
  PasswordField,
  SegmentedControl,
  Select,
  TextField,
} from "./components/ui";
import { CodeBlock } from "./components/CodeBlock";
import { Disclosure, SectionLabel } from "./features/connections/ui";
import {
  placementText,
  type ConnectionState,
  type Props,
} from "./McpConnectionState";

// Compact connection editor shown inside the inspector (`inline` mode).
type SectionProps = { form: ConnectionState; props: Props };

function InlineRunnerFields({ form }: { form: ConnectionState }) {
  const { input, patch, privateRunners } = form;
  return (
    <>
      {input.placement === "local" && (
        <TextField
          label="Runner reference"
          required
          maxLength={200}
          value={input.runner_reference ?? ""}
          onChange={(e) => patch({ runner_reference: e.currentTarget.value })}
        />
      )}
      {input.placement === "private" && (
        <Select
          label="Private runner"
          required
          value={input.runner_reference}
          data={(privateRunners.data ?? []).map((runner) => ({
            value: `private:${runner.id}`,
            label: `${runner.name} · ${runner.available ? "Connected" : "Offline"}`,
            disabled: !runner.eligible,
          }))}
          onChange={(value) => patch({ runner_reference: value })}
        />
      )}
      {privateRunners.error && input.placement === "private" && (
        <Alert variant="destructive">{privateRunners.error.message}</Alert>
      )}
    </>
  );
}

function InlineTargetFields({ form, props }: SectionProps) {
  const { input, patch, definition } = form;
  return (
    <>
      {!props.nameMount && (
        <TextField
          label="Connection name"
          required
          maxLength={120}
          value={input.name}
          onChange={(e) => patch({ name: e.currentTarget.value })}
        />
      )}
      <TextField
        label={
          definition.data?.summary.transport === "stdio"
            ? "Target label"
            : "Target"
        }
        required
        maxLength={2048}
        inputClassName="mono text-label"
        value={input.target}
        onChange={(e) => patch({ target: e.currentTarget.value })}
      />
      <Select
        label="Scope"
        clearable
        placeholder="Brain-wide"
        data={props.environments}
        value={input.environment_id}
        onChange={(value) => patch({ environment_id: value })}
      />
      <Select
        label="Runs on"
        required
        value={input.placement}
        data={(definition.data?.placements ?? [input.placement]).map(
          (value) => ({ value, label: placementText(value) }),
        )}
        onChange={(value) =>
          patch({ placement: value ?? "central", runner_reference: null })
        }
      />
      <InlineRunnerFields form={form} />
    </>
  );
}

function InlineSecretFields({ form }: { form: ConnectionState }) {
  const {
    owner,
    input,
    definition,
    authMode,
    setAuthMode,
    secrets,
    setSecrets,
  } = form;
  return (
    <>
      {owner &&
        input.placement === "central" &&
        !!definition.data?.credential_aliases.length && (
          <SegmentedControl
            label="Authentication"
            fullWidth
            value={authMode}
            onChange={(value) => {
              setAuthMode(value);
              setSecrets([]);
            }}
            data={[
              { value: "reference", label: "Credential reference" },
              { value: "secret", label: "Secret" },
            ]}
          />
        )}
      {authMode === "secret" && (
        <>
          {!secrets.length && (
            <TextField
              label={
                definition.data?.summary.transport === "stdio"
                  ? "Environment variable"
                  : "Header name"
              }
              placeholder={
                definition.data?.summary.transport === "stdio"
                  ? "MCP_TOKEN"
                  : "Authorization"
              }
              inputClassName="mono text-label"
              onChange={(e) =>
                setSecrets([
                  {
                    kind:
                      definition.data?.summary.transport === "stdio"
                        ? "environment"
                        : "header",
                    destination: e.currentTarget.value,
                    variable: "MCP_TOKEN",
                    prefix:
                      definition.data?.summary.transport === "stdio"
                        ? ""
                        : "Bearer ",
                    value: "",
                  },
                ])
              }
            />
          )}
          {secrets.map((secret, index) => (
            <div key={index} className="grid grid-cols-2 items-start gap-2">
              <TextField
                label={
                  secret.kind === "header"
                    ? "Header name"
                    : "Environment variable"
                }
                inputClassName="mono text-label"
                value={secret.destination}
                onChange={(e) => {
                  const destination = e.currentTarget.value;
                  setSecrets((values) =>
                    values.map((v, i) =>
                      i === index ? { ...v, destination } : v,
                    ),
                  );
                }}
              />
              <PasswordField
                label="Secret value"
                value={secret.value}
                onChange={(e) => {
                  const value = e.currentTarget.value;
                  setSecrets((values) =>
                    values.map((v, i) => (i === index ? { ...v, value } : v)),
                  );
                }}
              />
            </div>
          ))}
          <p className="m-0 text-caption text-muted-foreground">
            Saved separately in the installation-local credential file.
          </p>
        </>
      )}
    </>
  );
}

function InlineMoreConfiguration({ form, props }: SectionProps) {
  const { input, patch, definition, settings, setSettings, save } = form;
  return (
    <Disclosure summary="More configuration" bodyClassName="gap-3">
      <TextField
        label="Description"
        maxLength={2000}
        value={input.description}
        onChange={(e) => patch({ description: e.currentTarget.value })}
      />
      <Select
        label="Approved connector"
        required
        value={input.definition_key || null}
        data={props.catalogue.definitions.map((d) => ({
          value: d.key,
          label: d.name,
          disabled: !d.enabled,
        }))}
        onChange={(value) =>
          patch({
            definition_key: value ?? "",
            credential_alias: null,
          })
        }
      />
      <Select
        label="Credential alias"
        clearable
        placeholder="None selected"
        value={input.credential_alias}
        data={definition.data?.credential_aliases ?? []}
        onChange={(value) => patch({ credential_alias: value })}
      />
      <InlineSecretFields form={form} />
      <JsonInput
        label="Non-secret settings"
        value={settings}
        onChange={(value) => {
          setSettings(value);
          save.reset();
        }}
        validationError="Enter valid JSON"
        formatOnBlur
        autosize
        minRows={3}
      />
      {definition.data && (
        <Disclosure summary="Approved settings schema">
          <CodeBlock
            language="json"
            code={JSON.stringify(
              definition.data.configuration_schema,
              null,
              2,
            )}
          />
        </Disclosure>
      )}
      <Checkbox
        label="Connection enabled"
        checked={input.enabled}
        onChange={(e) => patch({ enabled: e.currentTarget.checked })}
      />
    </Disclosure>
  );
}

function InlineStatusAlerts({
  form,
  reload,
}: {
  form: ConnectionState;
  reload: () => void;
}) {
  const { definition, stepError, staleDraft, created, save } = form;
  return (
    <>
      {definition.error && (
        <Alert variant="destructive">{definition.error.message}</Alert>
      )}
      {stepError && <Alert variant="destructive">{stepError}</Alert>}
      {staleDraft && !created && (
        <Alert
          variant="warning"
          title="Connection changed"
          actions={
            <Button size="sm" variant="secondary" onClick={reload}>
              Reload settings
            </Button>
          }
        >
          Your draft is retained. Reload replaces it with current settings.
        </Alert>
      )}
      {save.error && (
        <Alert
          variant="destructive"
          title={
            created
              ? "Connection saved · credentials not saved"
              : "Connection not saved"
          }
        >
          {save.error.message}
        </Alert>
      )}
    </>
  );
}

function InlineFooter({ form, props }: SectionProps) {
  const { save, staleDraft, created, validateStep } = form;
  return (
    <div className="flex justify-end gap-2 border-t border-border pt-3">
      <Button
        variant="secondary"
        disabled={save.isPending}
        onClick={props.close}
      >
        Cancel
      </Button>
      <Button
        type="submit"
        loading={save.isPending}
        disabled={
          props.brain.archived ||
          (staleDraft && !created) ||
          [0, 1, 2, 3].some((index) => !!validateStep(index))
        }
      >
        {created ? "Retry credentials" : "Save connection"}
      </Button>
    </div>
  );
}

export function InlineConnectionForm({
  form,
  props,
  reload,
}: SectionProps & { reload: () => void }) {
  const { input, patch, save, setStepError, validateStep } = form;
  return (
    <form
      className="animate-enter"
      onSubmit={(e) => {
        e.preventDefault();
        const invalid = [0, 1, 2, 3].map(validateStep).find(Boolean);
        if (invalid) {
          setStepError(invalid);
          return;
        }
        save.mutate();
      }}
    >
      <fieldset disabled={save.isPending} className="m-0 min-w-0 border-0 p-0">
        {props.nameMount &&
          createPortal(
            <Input
              aria-label="Connection name"
              required
              className="font-medium"
              maxLength={120}
              disabled={save.isPending}
              value={input.name}
              onChange={(e) => patch({ name: e.currentTarget.value })}
            />,
            props.nameMount,
          )}
        <div className="grid gap-3">
          <SectionLabel>Configuration</SectionLabel>
          <InlineTargetFields form={form} props={props} />
          <InlineMoreConfiguration form={form} props={props} />
          <InlineStatusAlerts form={form} reload={reload} />
          <InlineFooter form={form} props={props} />
        </div>
      </fieldset>
    </form>
  );
}
