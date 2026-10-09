# Recollect Codex Setup

Start Codex from the Recollect repository root. The repository-local
[config.toml](config.toml) contains project MCP configuration and explicit role
registrations; [agents/](agents/) holds standalone role definitions. These
formats are supported by the [official configuration schema](https://learn.chatgpt.com/docs/config-schema.json)
and [agent documentation](https://learn.chatgpt.com/docs/agent-configuration/subagents).

Roles are default, explorer, architect, worker, qa, and reviewer. They inherit
personal model, reasoning, permission, and approval defaults. Instructions
limit explorer/reviewer to read-only work; this is not a sandbox override.
Delegate only when explicitly requested.

## Documentation skills

Two repository-local skills adapt the Terme workflows to Recollect:

| Skill | Use |
| --- | --- |
| [recollect-doc-router](../.agents/skills/recollect-doc-router/SKILL.md) | Find governing docs, owning epic/pack, current evidence, conflicts and the next authorized step |
| [recollect-doc-maintainer](../.agents/skills/recollect-doc-maintainer/SKILL.md) | Audit or update affected docs and reconcile lifecycle records and closeout |

Invoke them as `$recollect-doc-router` or `$recollect-doc-maintainer`, or let
Codex select them when the task matches their descriptions. Their UI metadata
lives beside each entrypoint in `agents/openai.yaml`; no MCP dependency or
home-level skill/configuration change is required. Both defer to docs/README.md
and current user scope. They do not import Terme's docsctl or lifecycle schema.

Codex discovers `.agents/skills/` between the current directory and the repository
root. Changes are detected automatically; reopen Codex if the skills do not
appear in its skill list. See [official skill guidance](https://learn.chatgpt.com/docs/build-skills)
and the [local assessment and verification](../docs/mappings/documentation-skills-and-tooling-2026-09-13.md).

## Context7

Prerequisites: Node.js/npm with `npx` on PATH. Supply `CONTEXT7_API_KEY` in the
environment of the process launching Codex. The config forwards its name only;
do not paste credentials into TOML. A root `.env` file is not automatically
loaded by this setup. Do not change home configuration to add this project MCP.

From this repository, inspect registration:

```sh
codex mcp get context7
```

This command proves configuration discovery only. In a fresh Codex session,
use `/mcp` to inspect connection status, then ask for a Context7 library lookup
to prove a successful read-only call. Existing sessions may need reopening to
load a newly created project configuration. Project configuration also depends
on the user's existing project trust settings; this repo does not modify them.

If configuration is absent, check the launch directory and project trust. If
startup or lookup fails, check Node/npx, network reachability, and environment
key presence without printing its value. Report the error accurately; offline
governance validation does not establish MCP connectivity.

The Context7 stanza matches both inspected reference repos and the supported
[stdio MCP configuration](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
Rust Analyzer and Graft are also registered as described below.

## Rust Analyzer MCP and Graft

Run `./scripts/setup-rust-navigation.sh` for the repository-local installation.
This setup currently supports Apple Silicon macOS with Rust, rust-src, Node/npm
and sandbox-exec. It installs a pinned Rust-written MCP bridge and standalone
rust-analyzer, and npm-locked Graft. Downloads, dependencies, analysis output
and indexes stay here; there is no home-level MCP installation.

Use Rust Analyzer for Rust definitions, references, types and diagnostics. Use
Graft for repository context, file APIs and structural call traces. Graft itself
is an upstream Node developer tool; Recollect's product remains Rust. Both local
runtime launchers deny networking and writes outside this checkout. Structural
queries need no model key or paid API call.

The project registrations locate this Git checkout from root or nested launch
directories. Inspect them with `codex mcp get rust_analyzer` and
`codex mcp get graft`. A fresh host session may be required to expose newly
registered tools. The [runbook](../docs/runbooks/rust-semantic-navigation.md)
contains the actual Rust stdio proof and refresh commands; the
[dated evidence](../docs/mappings/rust-semantic-navigation-proof-2026-10-09.md)
separates configuration discovery from successful calls and coverage limits.

## Local CodeGraph CLI

[tools/codegraph/](tools/codegraph/) contains an independent npm-locked
CodeGraph 1.6.0 installation and launcher. Run `./scripts/setup-codegraph.sh`
from Recollect, then:

```bash
./.codex/tools/codegraph/codegraph node get_authorized_dataset
```

No home configuration or session restart is required;
the launcher is available directly through the shell.

Recollect's root index explicitly includes the ignored `references/cognee/`
checkout and leaves its source/Git state untouched. The runtime, caches, and index stay
local and ignored; source manifests and settings remain reviewable. Refresh
with `sync` after source changes. See the
[runbook](../docs/runbooks/codegraph.md) and
[verification evidence](../docs/mappings/codegraph-setup-2026-09-13.md).
