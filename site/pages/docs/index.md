---
title: Documentation
description: Install Recollect, connect Codex, Claude Code or OpenCode, publish shared skills, and govern MCP tools and private runners.
---

# Give every coding agent the same memory.

Recollect is a self-hosted Brain for your coding agents. It captures what each
session learns, gives the next session cited context before its first prompt,
keeps your team's skills in sync across hosts, and decides which MCP tools an
agent may run, and where. These guides take you from a local install to a
shared, governed setup.

## Start in three steps

1. **[Install Recollect](./installation.md).** Start the complete local stack
   with Docker Compose, then sign in to the desktop app.
2. **[Connect your agent](./agent-memory-tools.md).** Install the plugin for
   Codex, Claude Code or OpenCode, connect it to a Brain, and launch your host
   normally. Cited context arrives before each prompt.
3. **Add what your team needs.** [Publish shared skills](./managed-skills.md),
   [create tool groups](./mcp-catalogue.md) and give people explicit Use
   permissions, or [pair a private runner](./mcp-vault-and-private-runners.md).

## Choose your next step

| I want to… | Guide |
| --- | --- |
| Set up automatic capture and recall | [Install the plugin](./plugin.md) and [session capture](./session-capture.md) |
| Decide what can be sent to AI | [AI policy and learning](./provider-learning.md) |
| Work across repositories and environments | [Workspace scope](./workspace-scope.md) |
| Publish skills and instructions to every host | [Shared skills and instructions](./managed-skills.md) |
| Control who can run which MCP tools | [Connections and tool groups](./mcp-catalogue.md) |
| Reach a filesystem or service inside a private network | [Private runners](./mcp-vault-and-private-runners.md) |
| Understand a tool call or recover its outcome | [Tool calls and outcomes](./mcp-runtime.md) |
| Let agents on different machines coordinate | [Peer coordination](./plugin.md#peer-coordination) |
| Explore code and knowledge relationships | [Graph exploration](./graph-exploration.md) |
| Manage retention, deletion and recovery | [Privacy](./retention-and-erasure.md) and [backup](./recovery.md) |

## The pieces, together

A **Brain** holds shared knowledge, skills and access rules. **Environments**
select the work context. **Skills** are Markdown guidance that every connected
host installs natively and loads on demand. **Connections** define approved MCP
targets and where they run. **Tool groups** give people or groups independent
Use, Manage and Share permissions. **Private runners** execute assigned calls
from paired devices inside your network.

Knowledge access and tool access are separate. Selecting an environment,
publishing a skill or registering a runner never grants permission to run tools.

::: info Project status
Recollect is an early project for individuals and trusted internal teams.
The documented workflows have local acceptance evidence. Public marketplace
distribution, native MCP OAuth and further evaluation remain separate work.
See [current work](https://github.com/MikeK184/Recollect/blob/main/docs/roadmap/epics/index.md)
and [evaluation limits](./public-memory-benchmark.md).
:::

These guides are built from the repository's canonical Markdown. Architecture,
contracts and dated evidence remain available in the
[repository documentation](https://github.com/MikeK184/Recollect/tree/main/docs).
