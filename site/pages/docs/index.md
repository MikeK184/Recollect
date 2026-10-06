---
title: Documentation
description: Install Recollect, connect a coding agent, build shared knowledge and govern MCP tools and private runners.
---

# Build a Brain your agents can work with.

Recollect brings shared engineering memory, governed MCP tools and private
execution together. Start with your own installation, connect your coding host,
and let approved capture and recall carry context into the next session.

## Start in three steps

1. **[Install Recollect](./installation.md).** Start the complete local stack
   with Docker Compose, then open the desktop UI.
2. **[Connect your agent](./agent-memory-tools.md).** Install the plugin for
   Codex, Claude Code or OpenCode, connect to a Brain, and launch your host normally.
3. **[Add tools when you need them](./mcp-catalogue.md).** Create approved
   connections and tool groups, then give callers explicit Use permissions.

## Choose your next step

| I want to… | Guide |
| --- | --- |
| Set up automatic capture and recall | [Install the plugin](./plugin.md) |
| Work across repositories and environments | [Workspace scope](./workspace-scope.md) |
| Decide what can be sent to AI | [AI policy and learning](./provider-learning.md) |
| Control who can run which MCP tools | [Connections and tool groups](./mcp-catalogue.md) |
| Reach a filesystem or service inside a private network | [Private runners](./mcp-vault-and-private-runners.md) |
| Understand a tool call or recover its outcome | [Tool runtime](./mcp-runtime.md) |
| Explore code and knowledge relationships | [Graph exploration](./graph-exploration.md) |
| Manage retention, deletion and recovery | [Privacy](./retention-and-erasure.md) and [backup](./recovery.md) |

## The pieces, together

A **Brain** holds shared knowledge and its access rules. **Environments** select
the work context. **Connections** define approved MCP targets and their execution
location. **Tool groups** give people or groups independent Use, Manage and Share
permissions. **Private runners** execute assigned calls from paired devices.

Knowledge access and tool access are separate. Selecting an environment or
registering a runner does not grant permission to execute tools.

::: info Project status
Recollect is an early project for individuals and trusted internal teams.
The documented workflows have local acceptance evidence. Public marketplace
distribution, native MCP OAuth and further evaluation remain separate work.
See [current work](https://github.com/MikeK184/Recollect/blob/main/docs/roadmap/execution/active/README.md)
and [evaluation limits](./public-memory-benchmark.md).
:::

These guides are built from the repository's canonical Markdown. Architecture,
contracts and dated evidence remain available in the
[repository documentation](https://github.com/MikeK184/Recollect/tree/main/docs).
