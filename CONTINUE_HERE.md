# Continue Recollect work

Read [docs/README.md](docs/README.md), then the
[current product status and remaining features](docs/roadmap/epics/index.md).
That roadmap is the single status entry point; this file holds no status,
handoff or chronology.

Work is tracked as epic slice rows with acceptance and validation inline. The
[active packs](docs/roadmap/execution/active/README.md) exist only for risky
unfinished slices; each is the starting point for its slice, and Git and
runtime state are rechecked before work continues. Shipped packs and earlier
handoffs are in Git history, for example
[CONTINUE_HERE.md at checkpoint f2673d7](https://github.com/MikeK184/Recollect/blob/18a8ba7ee8399f143b6e22d16c71c81b2a608ae0/CONTINUE_HERE.md).

Navigation uses [Rust Analyzer MCP and Graft](docs/runbooks/development-tooling.md)
for semantic and structural lookups and `rg` for literal, configuration and docs
discovery.
