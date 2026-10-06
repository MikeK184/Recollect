# Owned Ubuntu private-runner demo

This explicitly selected local lab adds one runner, read-only filesystem
connection and tool group to **SWEG — test**. The existing authorized `demo`
reader calls the Brain's real agent MCP interface; Ubuntu owns a separate
paired identity. See [dated proof](../../docs/mappings/private-runner-ubuntu-proof-2026-10-06.md).

Prerequisites are the ready local installation, owner variables in the ignored
root `.env`, Docker, OpenSSL and the existing ignored test caller credential
`.data/runtime/demo-agent.json`. This is a local lab procedure, not a shared
installation bootstrap. It intentionally refuses unknown resource collisions.

```sh
python3 scripts/prepare-private-runner-demo.py
python3 scripts/prove-private-runner-demo.py
```

Preparation uses a marked repository cache and preserves its private TLS and
unlock material. Proof uses native pairing into Ubuntu Secret Service, then
canonical registration/definition/connection/profile APIs. Only the new
`Ubuntu demo files` group gets explicit Use for `owner` and `demo`; the reader's
Manage/Share and existing groups stay unchanged. The registered device and
approved executable/argv are fixed; model arguments cannot select a shell.

The filesystem package exposes many tools internally, but this definition
approves only `read_text_file`, `list_directory` and `list_allowed_directories`.
The synthetic `/srv/recollect-demo` folder has read-only permissions. No host
folders or Docker socket are mounted; the only host bind is this lab's TLS
directory. The OS store/receipts use one owned named volume, all capabilities
are dropped, and no ports are published. The TLS relay is confined to Ubuntu
loopback and forwards to the existing Mac local API; a real office deployment
uses its actual reachable HTTPS Recollect URL.

Inspect **SWEG — test → Connections → Private Runners** or its new filesystem
connection. Stop/start the retained demo without touching other services:

```sh
./scripts/docker.sh stop recollect-ubuntu-private-runner-demo
./scripts/docker.sh start recollect-ubuntu-private-runner-demo
```

The same container retains its selected runner ID. A fresh container needs the
same OS-store volume, private runtime/TLS files and registered runner ID written
to `/run/recollect/runner-id`; the fresh-container proof exercised this path.
The fixture scripts do not automatically replace containers or rotate credentials.
After a completed run, `--fresh` repeats read/denial checks with a new immutable
caller task/session against the retained resources. The original offline proof
is retained, not rerun. Evidence goes to `output/private-runner-ubuntu-2026-10-06`.
The disposable certificate lasts 30 days and device credentials follow the
product's normal expiry; this demo is not an unattended production service.
