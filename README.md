# Recollect

Memory and MCP tools for coding agents, with a desktop web UI.

Recollect keeps repository knowledge, documents, decisions and runbooks together
so your next coding session can pick up where the last one left off. It runs on
your own machine or a private server and connects to Codex and Claude Code.

## What it does

- Organizes knowledge into **Brains**, with access controls for individuals and teams.
- Captures supported agent sessions and tool results, with sanitization and offline delivery.
- Learns, revises and retires memories automatically under your chosen policies;
  manual review and corrections are optional.
- Searches using exact matches, keywords, embeddings and graph relationships,
  with links back to the evidence.
- Coordinates approved MCP tools locally, centrally or through private runners.
  Vault is an optional credential source.
- Provides a desktop UI for exploring memory, graphs, processing and tool calls.

Built with Rust, React, PostgreSQL/pgvector and Neo4j/GDS Community.

## Run locally

Install Rust/Cargo, Node.js/npm, Python 3.11+ and Docker with Compose, then run:

```sh
git clone https://github.com/MikeK184/Recollect.git
cd Recollect
./scripts/dev.sh
```

Open **http://127.0.0.1:8787**. The first run generates login credentials in the
ignored `.env` file: use `RECOLLECT_OWNER_USERNAME` and `RECOLLECT_OWNER_PASSWORD`.
The script builds the application and starts its databases, API and worker.

Learning and semantic search require an `OPENAI_API_KEY` in `.env` and an enabled
Brain model policy. Model requests are billed by the provider. Each Brain starts
with model transmission disabled.

For Docker installation and shared HTTPS access, see the
[installation guide](docs/runbooks/installation.md).

## Status and documentation

This is an early project for local and trusted private use. The first planned
product scope is implemented and locally tested; desktop is the current UI target.
The [evaluation report](docs/mappings/integrated-evaluations-2026-09-26.md)
records measured results, failures and limits. It is not a production capacity guarantee.

- [Local development](docs/runbooks/local-development.md)
- [Connect coding agents](docs/runbooks/agent-memory-tools.md)
- [Operation guides](docs/runbooks/README.md)
- [Architecture](docs/foundation/README.md)

## Contributing

Issues and focused pull requests are welcome. Include relevant tests for behavior
changes and run `./scripts/validate.sh` for the documentation checks. See
[AGENTS.md](AGENTS.md) for repository conventions and the development guide for
product tests.

## License

Licensed under [Apache-2.0](LICENSE).
