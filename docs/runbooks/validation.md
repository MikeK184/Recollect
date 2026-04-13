# Governance Validation

## Purpose and Prerequisites

Run Bash and Python 3.11+ from any working directory using the script's absolute
path, or run this from the repository root:

```sh
./scripts/validate.sh
```

The command uses only the Python standard library. It validates governed files
and runs checker tests in disposable temporary directories. It does not install
packages, call a network service, traverse Cognee, or select an application stack.

## Checks

- Required documents, templates, metadata, sections, and local Markdown links.
- Accepted authority links; pending foundations reported and rejected as
  governing sources; product packs cannot start or ship against a pending baseline.
- Unique epic/slice/pack identities, exact owning-epic relationship, statuses,
  required packs, and active/archive plus epic index agreement.
- Explicit execution requirements and documented small-fix exemptions.
- Filled implementation fields and shipped closeout evidence fields.
- TOML syntax, role names/registration targets, inherited personal defaults,
  and secret-free Context7 configuration.
- Parent-repo ignore rule for the separate Cognee checkout.

The checker accepts the documented simple Markdown tables and inline links.
Use one row per record without pipe characters inside table cells. Metadata
uses one `Key: value` line above the first second-level heading. It checks local
file targets, not heading anchors or remote URL availability. It does not judge
semantic completeness, factual accuracy, or implementation correctness.

## Expected Result and Recovery

Success prints `Governance lint passed`, reports any pending foundations, and
runs the fixture tests successfully. Failure prints a file and actionable
reason, returning nonzero. Correct the governing record and all affected
indexes together, then rerun. Do not weaken the checker to hide unfinished work.

For only the current documentation/configuration check:

```sh
./docs/tools/lint_agent_governance.sh
```

There is no CI host or remote configured yet. This local command is the future
CI entrypoint. Verify Context7 separately using the
[Codex setup procedure](../../.codex/README.md).
