# Continue Recollect work

Read [docs/README.md](docs/README.md), then the
[current product status and remaining features](docs/roadmap/epics/index.md).
That roadmap is the current status entry point; this file does not maintain a
second chronological handoff.

Product implementation was paused on 2026-10-09 after checkpoint
`f2673d71c4b63b536d41b3143eb39c207955b201` was committed and pushed. The
2026-10-18 documentation cleanup does not deploy code. Separate graph
implementation has since resumed; use the current roadmap and active packs
for its scope and acceptance.

The [active packs](docs/roadmap/execution/active/README.md) retain graph
reliability, automatic organization and native-host acceptance. Start from the
owning pack and recheck Git/runtime state before continuing. Dated
[benchmark evidence](docs/mappings/openrouter-benchmark-proof-2026-10-08.md) and
[implementation checkpoint](docs/mappings/automatic-knowledge-mapping-progress-2026-10-09.md)
retain the measured results and limits.

Use [Rust Analyzer MCP and Graft](docs/runbooks/rust-semantic-navigation.md)
for semantic/structural navigation and `rg` for literal/config/docs discovery.
Historical handoffs are available in
[Git](https://github.com/MikeK184/Recollect/blob/f2673d71c4b63b536d41b3143eb39c207955b201/CONTINUE_HERE.md).
