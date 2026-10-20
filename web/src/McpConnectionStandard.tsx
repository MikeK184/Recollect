import { Link } from "@tanstack/react-router";
import { ArrowRight, Layers } from "lucide-react";
import {
  Alert,
  Button,
  Checkbox,
  JsonInput,
  PasswordField,
  SegmentedControl,
  Select,
  Spinner,
  TextField,
} from "./components/ui";
import { CodeBlock } from "./components/CodeBlock";
import { Disclosure, SectionLabel } from "./features/connections/ui";
import {
  placementText,
  type ConnectionState,
  type Detail,
  type DefinitionDetail,
  type Props,
} from "./McpConnectionState";

// Full connection form used by the dialog (non-inline mode).
type SectionProps = { form: ConnectionState; props: Props };
type DefinitionSectionProps = { form: ConnectionState; def: DefinitionDetail };

function ConnectorFields({ form, props }: SectionProps) {
  const { input, patch } = form;
  return (
    <>
      <TextField
        label="Connection name"
        required
        maxLength={120}
        value={input.name}
        onChange={(e) => patch({ name: e.currentTarget.value })}
      />
      <TextField
        label="Connection description"
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
          label: `${d.name}${d.enabled ? "" : " · disabled"}`,
        }))}
        onChange={(v) =>
          patch({ definition_key: v ?? "", credential_alias: null })
        }
      />
    </>
  );
}

function TargetFields({
  form,
  props,
  def,
}: SectionProps & { def: DefinitionDetail }) {
  const { input, patch } = form;
  return (
    <>
      <Select
        label="Connection environment"
        clearable
        placeholder="Brain-wide"
        value={input.environment_id}
        data={props.environments}
        onChange={(v) => patch({ environment_id: v })}
      />
      <Button variant="ghost" size="sm" className="-ml-2.5 justify-self-start" asChild>
        <a href={`/brains/${props.brain.id}/settings?tab=general#environments`}>
          <Layers aria-hidden="true" />
          Manage optional environments
        </a>
      </Button>
      <TextField
        label={
          def.summary.transport === "streamable_http"
            ? "MCP target URL"
            : "Target label"
        }
        required
        maxLength={2048}
        value={input.target}
        onChange={(e) => patch({ target: e.currentTarget.value })}
      />
    </>
  );
}

function PrivateRunnerFields({ form, props }: SectionProps) {
  const { input, patch, privateRunners } = form;
  return (
    <>
      {privateRunners.error && (
        <Alert
          variant="destructive"
          actions={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => void privateRunners.refetch()}
            >
              Refresh private runners
            </Button>
          }
        >
          {privateRunners.error.message}
        </Alert>
      )}
      {!privateRunners.error && privateRunners.data?.length === 0 && (
        <Alert variant="warning" role="status">
          This Brain has no private runner. Register one for a paired device on
          the Private Runners tab first; until then this target has nothing that
          can reach it.
        </Alert>
      )}
      <Select
        label="Private runner"
        required
        placeholder="Select this Brain's runner"
        value={input.runner_reference}
        data={[
          ...(privateRunners.error ? [] : (privateRunners.data ?? [])).map(
            (r) => ({
              value: `private:${r.id}`,
              label: `${r.name} · ${!r.eligible ? "Unavailable" : r.available ? "Connected" : "Offline"}`,
              disabled:
                !r.eligible && input.runner_reference !== `private:${r.id}`,
            }),
          ),
          ...(input.runner_reference &&
          !(privateRunners.data ?? []).some(
            (r) => input.runner_reference === `private:${r.id}`,
          )
            ? [
                {
                  value: input.runner_reference,
                  label: "Previously selected runner · unavailable",
                },
              ]
            : []),
        ]}
        onChange={(value) => patch({ runner_reference: value })}
      />
      <p className="m-0 text-label text-muted-foreground">
        Offline runners keep calls queued until the runner connects or the queue
        deadline expires. A registration is metadata: it proves nothing until a
        call succeeds there.
      </p>
      <p className="m-0 text-label text-muted-foreground">
        Each runner is bound to one paired device. Register or rename it in
        execution administration, and manage the device itself in{" "}
        <Link
          to="/brains/$brainId/agents"
          params={{ brainId: props.brain.id }}
          className="text-brand-text underline-offset-[3px] hover:underline"
        >
          this Brain&apos;s Agents list
        </Link>
        .
      </p>
    </>
  );
}

function PlacementFields({
  form,
  props,
  def,
}: SectionProps & { def: DefinitionDetail }) {
  const { input, patch } = form;
  return (
    <>
      <Select
        label="Where this connection runs"
        description="This decides which network can reach the target. The chosen place never falls back to another."
        required
        value={input.placement}
        data={def.placements.map((p) => ({
          value: p,
          label: placementText(p),
        }))}
        onChange={(v) =>
          patch({
            placement: v ?? "central",
            runner_reference: null,
          })
        }
      />
      {props.manageRunners && input.placement === "private" && (
        <Button
          variant="ghost"
          size="sm"
          className="-ml-2.5 justify-self-start"
          onClick={props.manageRunners}
        >
          Manage private-network execution
          <ArrowRight aria-hidden="true" />
        </Button>
      )}
      {input.placement === "local" && (
        <TextField
          label="Runner reference"
          description="Name the runner on your paired device. Offline devices apply nothing."
          required
          maxLength={200}
          value={input.runner_reference ?? ""}
          onChange={(e) => patch({ runner_reference: e.currentTarget.value })}
        />
      )}
      {input.placement === "private" && (
        <PrivateRunnerFields form={form} props={props} />
      )}
    </>
  );
}

function SecretFields({ form, def }: DefinitionSectionProps) {
  const { secrets, setSecrets } = form;
  return (
    <>
      {!secrets.length && (
        <div className="grid">
          <TextField
            label={
              def.summary.transport === "stdio"
                ? "Environment variable"
                : "Header name"
            }
            placeholder={
              def.summary.transport === "stdio" ? "MCP_TOKEN" : "Authorization"
            }
            onChange={(e) =>
              setSecrets([
                {
                  kind:
                    def.summary.transport === "stdio"
                      ? "environment"
                      : "header",
                  destination: e.currentTarget.value,
                  variable: "MCP_TOKEN",
                  prefix: def.summary.transport === "stdio" ? "" : "Bearer ",
                  value: "",
                },
              ])
            }
          />
        </div>
      )}
      {secrets.map((secret, i) => (
        <div key={i} className="grid grid-cols-2 items-start gap-3">
          <TextField
            label={
              secret.kind === "header" ? "Header name" : "Environment variable"
            }
            value={secret.destination}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setSecrets((values) =>
                values.map((v, j) =>
                  j === i ? { ...v, destination: value } : v,
                ),
              );
            }}
          />
          <PasswordField
            label={secret.prefix ? "Bearer token" : "Secret value"}
            value={secret.value}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setSecrets((values) =>
                values.map((v, j) => (j === i ? { ...v, value } : v)),
              );
            }}
          />
        </div>
      ))}
      <p className="m-0 text-caption text-muted-foreground">
        Values are saved separately in the installation-local development
        credential file. They are never stored in this connection’s settings.
      </p>
    </>
  );
}

function AuthenticationFields({ form, def }: DefinitionSectionProps) {
  const {
    owner,
    input,
    patch,
    authMode,
    setAuthMode,
    setSecrets,
    settings,
    setSettings,
    save,
  } = form;
  return (
    <>
      <SectionLabel className="mt-2">Authentication</SectionLabel>
      {owner &&
        input.placement === "central" &&
        def.credential_aliases.length > 0 && (
          <SegmentedControl
            label="Authentication"
            className="justify-self-start"
            value={authMode}
            onChange={(v) => {
              setAuthMode(v);
              setSecrets([]);
            }}
            data={[
              { value: "reference", label: "Credential reference" },
              { value: "secret", label: "Secret" },
            ]}
          />
        )}

      <Select
        label="Credential alias"
        clearable
        placeholder="No credential selected"
        value={input.credential_alias}
        data={def.credential_aliases}
        onChange={(v) => patch({ credential_alias: v })}
      />
      {authMode === "secret" && <SecretFields form={form} def={def} />}
      <JsonInput
        label="Non-secret settings"
        description="Use only settings permitted by the approved schema. Credential values belong in the credential provider."
        value={settings}
        onChange={(v) => {
          setSettings(v);
          save.reset();
        }}
        validationError="Enter valid JSON"
        formatOnBlur
        autosize
        minRows={4}
      />
      <Disclosure summary="Approved settings schema">
        <CodeBlock
          language="json"
          code={JSON.stringify(def.configuration_schema, null, 2)}
        />
      </Disclosure>
    </>
  );
}

function DefinitionSections({
  form,
  props,
  def,
}: SectionProps & {
  def: DefinitionDetail;
}) {
  const { input, patch } = form;
  return (
    <>
      <p className="m-0 rounded-md border border-border bg-muted px-3 py-2 text-label text-foreground-secondary">
        {def.summary.description} ·{" "}
        <span className="mono text-caption">{def.summary.transport}</span>
      </p>
      {!def.summary.enabled && (
        <Alert variant="warning" role="status">
          This definition is disabled. Enablement cannot be saved until an
          approved definition is selected.
        </Alert>
      )}
      <SectionLabel className="mt-2">Target</SectionLabel>
      <TargetFields form={form} props={props} def={def} />
      <SectionLabel className="mt-2">Execution</SectionLabel>
      <PlacementFields form={form} props={props} def={def} />
      <AuthenticationFields form={form} def={def} />
      <Checkbox
        label="Connection enabled"
        description="Allow future tool calls. Disable to pause use; dispatched calls may finish."
        checked={input.enabled}
        onChange={(e) => patch({ enabled: e.currentTarget.checked })}
      />
    </>
  );
}

function StandardFooter({ form, props }: SectionProps) {
  const { save, input, authMode, secrets, validateStep } = form;
  return (
    <div className="sticky -bottom-5 z-[2] -mx-5 -mb-5 mt-2 flex justify-end gap-2 border-t border-border bg-muted px-5 py-3">
      <Button variant="secondary" onClick={props.close}>
        Close
      </Button>
      <Button
        type="submit"
        loading={save.isPending}
        disabled={
          props.brain.archived ||
          [0, 1, 2, 3].some((index) => !!validateStep(index)) ||
          (authMode === "secret" &&
            (!input.credential_alias ||
              input.placement !== "central" ||
              !secrets.length ||
              secrets.some((s) => s.value.length < 4)))
        }
      >
        {"Save connection"}
      </Button>
    </div>
  );
}

export function StandardConnectionForm({
  form,
  props,
  detail,
}: SectionProps & { detail?: Detail }) {
  const { stepError, definition, save, setStepError, validateStep } = form;
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const invalid = [0, 1, 2, 3]
          .map((index) => ({ index, error: validateStep(index) }))
          .find((item) => item.error);
        if (invalid) {
          setStepError(invalid.error);
          return;
        }
        save.mutate();
      }}
    >
      <div className="grid gap-3">
        {stepError && (
          <Alert variant="destructive" title="Complete this connection">
            {stepError}
          </Alert>
        )}
        <p className="m-0 text-body text-muted-foreground">
          Choose an approved connector, its target and execution location.
          Saving does not call a tool.
        </p>
        {detail && detail.profile_ids.length > 0 && (
          <Alert variant="warning" role="status">
            This connection is used by {detail.profile_ids.length}{" "}
            {detail.profile_ids.length === 1 ? "profile" : "profiles"}. Changes
            affect their subsequent discovery and calls.
          </Alert>
        )}
        <SectionLabel className="mt-1">Connector</SectionLabel>
        <ConnectorFields form={form} props={props} />
        {definition.isFetching && !definition.data && (
          <Spinner label="Loading connector" />
        )}
        {definition.error && (
          <Alert variant="destructive">{definition.error.message}</Alert>
        )}
        {definition.data && !definition.error && (
          <DefinitionSections form={form} props={props} def={definition.data} />
        )}
        {save.error && (
          <Alert variant="destructive" title="Connection not saved">
            {save.error.message}
          </Alert>
        )}
        <StandardFooter form={form} props={props} />
      </div>
    </form>
  );
}
