# Live Ubuntu private-runner filesystem proof

Observed: 2026-10-06
Confidence: verified

## Sources and Method

- Existing ready `http://127.0.0.1:8787` installation and SWEG — test Brain `5c054930-d266-4c18-a42b-942729f942aa`.
- [Official filesystem MCP](https://github.com/modelcontextprotocol/servers/blob/main/src/filesystem/README.md), npm registry `@modelcontextprotocol/server-filesystem` version `2026.8.31`, installed SDK `1.32.1`; Ubuntu `24.04.5 LTS`, Node 22.
- Context7 lookup did not identify the official filesystem server; official repository/registry and installed package/runtime supplied the evidence. GitHub main package version differs from the dated npm release; the actually installed npm package is recorded.
- [Fixture source/procedure](../../infra/private-runner-demo/README.md), [preparation](../../scripts/prepare-private-runner-demo.py) and [real proof](../../scripts/prove-private-runner-demo.py).
- [Sanitized resources/checks](../../output/private-runner-ubuntu-2026-10-06/resources.json), [agent discovery](../../output/private-runner-ubuntu-2026-10-06/discovery.json), [returned file](../../output/private-runner-ubuntu-2026-10-06/read-marker.json), [no-Use denial](../../output/private-runner-ubuntu-2026-10-06/no-use-denial.json), [outside-root denial](../../output/private-runner-ubuntu-2026-10-06/outside-root-denial.json), [offline queue](../../output/private-runner-ubuntu-2026-10-06/offline-queued.json) and [same-call recovery](../../output/private-runner-ubuntu-2026-10-06/offline-recovered.json).
- Actual CUA [Connected card](../../output/private-runner-ubuntu-2026-10-06/private-runner-connected.png) and [private connection](../../output/private-runner-ubuntu-2026-10-06/filesystem-connection.png); no intercepted/fixture browser responses.

## Observations

1. Native `recollect-agent pair` stored the runner's credential in Ubuntu Secret Service. Runner device `f5dd4d6c-f4bf-405d-91de-776bae0a1e67` differs from demo caller `d7eaafe6-beab-4b8d-89dd-b36bdea46843`.
2. Registration `06f572a4-5865-4472-9a0e-4299e3860363`, connection `843f47a8-4196-47da-a439-1647c5636bed` and group `2fe52917-4077-4f89-99b7-1e2666ab0aa7` are new owned demo records. Existing Context7/Exa connections, profiles and grants were preserved.
3. The real agent HTTP MCP interface negotiated `2025-11-25`. Reader task/immutable tool operation, discovery, call and status used actual MCP JSON-RPC. Reader catalogue omitted `memory.contribute`.
4. Before explicit new-group Use, submission failed with `mcp_permission_required`. After Use, demo remained Brain reader with Manage/Share false; actual management and grant escalation requests both returned 403. Owner also received explicit Use on this new group only.
5. Exactly three approved read tools were discovered. `read_text_file` returned the marker generated inside `/srv/recollect-demo/hello.txt`; `list_directory` returned `hello.txt`; `list_allowed_directories` reported `/srv/recollect-demo`. The approved command/argv started the official stdio provider through the native runner.
6. `/etc/passwd` returned a provider tool error for a path outside allowed directories. `write_file` was rejected with `invalid_input` before dispatch because it is not approved. Reader output did not publish Brain knowledge.
7. After stopping the named runner and waiting for its lease to expire, call `554647e2-a9b5-4aaa-bc95-51b74bd703f3` stayed queued, undispatched, for the same private reference. Restart returned the marker on that same call; no central takeover occurred.
8. A fresh container from the corrected image reused the owned OS-store volume without pairing again and passed new reads, directory-boundary and write/authority denials. Runtime and current tag matched; exact image and package versions are in the resource ledger.
9. Live Private Runners showed Connected/Enabled; Connections showed Brain-wide, Private runner, Last call succeeded and three approved tools. Its Test dialog correctly directed device connections to a permitted tool call; the actual Tools dialog loaded the approved group.

The retained container is `recollect-ubuntu-private-runner-demo`, using one
owned Secret Service/receipt volume and a read-only bind of its private TLS
directory. No customer/Mac folders or Docker socket are mounted, no ports are
published, all Linux capabilities are dropped and no-new-privileges is enabled.
The fixture initializes a read-only synthetic demo folder. Its loopback TLS
relay reaches the existing local Mac API; native certificate validation remains
enabled with a dedicated trusted CA. Shared office use requires the actual HTTPS
Recollect address, not this local-only relay.

Initial fixture-only failures were repaired: Docker's legacy builder rejected
BuildKit cache mounts; a CA-as-server certificate failed native TLS validation;
the read-only marker needed restart-safe initialization and the copied startup
script needed executable mode. The corrected image/fresh-container proof passed.
Expected permission errors were matched to their actual bounded product codes.

## Translation and Limits

This proves **agent MCP caller → Recollect authorization/queue → Ubuntu native
private runner → official filesystem MCP → result returned to the caller**.
It is a protocol-client proof, not a fresh OpenCode/Codex LLM session and does
not close the separate native-token host pack. No paid model acceptance,
customer-network access, arbitrary shell execution, write tools or production
service operation was tested. Frontend/backend deployment and schema were unchanged.

Focused native build and real success/denial/offline/fresh-container checks pass;
Python compilation, shell syntax, preparation rerun, required governance32,
CodeGraph and whitespace pass. Demo caller tasks are closed and sessions released;
the named runner and new demo configuration remain online for inspection.
Original browser tab/session and unrelated Docker services are preserved.

## Follow-up

Stop/start only the named demo as described in its procedure. Its disposable
certificate and normal device credential expire; this is not an unattended
production deployment. Verify native host/model selection separately before
claiming an OpenCode LLM chose the filesystem tool. Version N/A; uncommitted.
