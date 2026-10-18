import { createPortal } from "react-dom";
import {
  Alert,
  Button,
  Checkbox,
  Group,
  JsonInput,
  Select,
  Stack,
  PasswordInput,
  SegmentedControl,
  Text,
  TextInput,
} from "@mantine/core";
import { CodeBlock } from "./components/CodeBlock";
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
        <TextInput
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
        <Alert color="red">{privateRunners.error.message}</Alert>
      )}
    </>
  );
}

function InlineTargetFields({ form, props }: SectionProps) {
  const { input, patch, definition } = form;
  return (
    <>
      {!props.nameMount && (
        <TextInput
          label="Connection name"
          required
          maxLength={120}
          value={input.name}
          onChange={(e) => patch({ name: e.currentTarget.value })}
        />
      )}
      <TextInput
        label={
          definition.data?.summary.transport === "stdio"
            ? "Target label"
            : "Target"
        }
        required
        maxLength={2048}
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
            <TextInput
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
            <Group key={index} grow align="start">
              <TextInput
                label={
                  secret.kind === "header"
                    ? "Header name"
                    : "Environment variable"
                }
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
              <PasswordInput
                label="Secret value"
                value={secret.value}
                onChange={(e) => {
                  const value = e.currentTarget.value;
                  setSecrets((values) =>
                    values.map((v, i) => (i === index ? { ...v, value } : v)),
                  );
                }}
              />
            </Group>
          ))}
          <Text size="xs" c="dimmed">
            Saved separately in the installation-local credential file.
          </Text>
        </>
      )}
    </>
  );
}

function InlineMoreConfiguration({ form, props }: SectionProps) {
  const { input, patch, definition, settings, setSettings, save } = form;
  return (
    <details className="inspector-more-configuration">
      <summary>More configuration</summary>
      <Stack mt="sm" gap="sm">
        <TextInput
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
          <details>
            <summary>Approved settings schema</summary>
            <CodeBlock
              language="json"
              code={JSON.stringify(
                definition.data.configuration_schema,
                null,
                2,
              )}
            />
          </details>
        )}
        <Checkbox
          label="Connection enabled"
          checked={input.enabled}
          onChange={(e) => patch({ enabled: e.currentTarget.checked })}
        />
      </Stack>
    </details>
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
        <Alert color="red">{definition.error.message}</Alert>
      )}
      {stepError && <Alert color="red">{stepError}</Alert>}
      {staleDraft && !created && (
        <Alert color="yellow" title="Connection changed">
          Your draft is retained. Reload replaces it with current settings.
          <Button variant="subtle" size="xs" onClick={reload}>
            Reload settings
          </Button>
        </Alert>
      )}
      {save.error && (
        <Alert
          color="red"
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
    <Group justify="end" className="connection-setup-footer">
      <Button variant="default" disabled={save.isPending} onClick={props.close}>
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
    </Group>
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
      className="connection-inline-editor"
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
      <fieldset disabled={save.isPending} className="mcp-inline-fields">
        {props.nameMount &&
          createPortal(
            <TextInput
              aria-label="Connection name"
              required
              className="mcp-inline-title"
              maxLength={120}
              disabled={save.isPending}
              value={input.name}
              onChange={(e) => patch({ name: e.currentTarget.value })}
            />,
            props.nameMount,
          )}
        <Stack gap="sm">
          <h3 className="management-title inspector-section-title">
            Configuration
          </h3>
          <InlineTargetFields form={form} props={props} />
          <InlineMoreConfiguration form={form} props={props} />
          <InlineStatusAlerts form={form} reload={reload} />
          <InlineFooter form={form} props={props} />
        </Stack>
      </fieldset>
    </form>
  );
}
