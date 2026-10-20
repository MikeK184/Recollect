# Skills Provenance and Sync

Status: planned

## Purpose

Know where every shared skill comes from and keep it current everywhere: record
each revision's upstream source, check that source for updates an administrator
approves, target skills at environments, people and agents, and show which host
holds which revision. Part 5 of the five-part improvement sequence in the [roadmap](index.md#improvement-sequence).

## Governing Sources

- [ADR 0022: Brain-managed skills](../../adr/0022-brain-managed-skills.md)
- [Managed skills](../../contracts/mcp-managed-skills.md) and
  [native skill reload](../../contracts/mcp-native-skill-reload.md)
- [Team access](../../contracts/platform-team-access.md) and
  [plugin session memory](../../contracts/mcp-plugin-session-memory.md)

## Dependencies and Boundaries

Runs alongside the other parts. Source provenance, upstream updates and the drift
report can start any time after [memory quality and proof](memory-quality-and-proof.md);
targeting agents waits for `agent-identity-principals` in
[agent identity and authority](agent-identity-and-authority.md). URL and Git
import is deferred by ADR 0022 today, so provenance and updates need an ADR 0022
amendment first; upstream updates are a new external integration and need a pack.
Publication stays a Brain-administrator action and nothing auto-publishes.

**Open decisions for the owner.**
- Is skill targeting by account, group or agent an organization view (like environment applicability today) or a confidentiality boundary?
- Which upstream skill sources are allowed (any HTTPS Git host, or an installation allowlist), and is a check for updates scheduled or only on demand?

## Slice Map

| Slice ID | Status | Evidence | Execution | Summary |
| --- | --- | --- | --- | --- |
| `skills-source-provenance` | planned | needs-adr, needs-contract | inline: every bundle revision may record its upstream source (HTTPS URL or Git remote, ref, resolved commit, path, licence and content hash), shown in the Agents surface and in generated native skill provenance, while publication stays a Brain-administrator action; validation: HTTP, MCP and native-export tests showing provenance on each revision and rejecting credentials in URLs | Answers "where does this skill come from"; needs an ADR 0022 amendment because URL and Git import is deferred today |
| `skills-upstream-updates` | planned | needs-adr, needs-contract | inline: a bounded check of the recorded source finds a newer commit or tag, shows a file diff and lets a Brain administrator publish it as a new immutable revision, never auto-publishing or running scripts; validation: fixture Git and HTTPS sources covering update, unchanged, removed, oversized, redirect and licence-change cases | Keeps shared skills current; new external integration, so a pack is required |
| `skills-revision-drift-report` | planned | needs-contract | inline: administrators see which connected hosts and sessions hold which bundle revision and which copies are stale, from plugin synchronization reports that carry no secrets or local paths beyond workspace identity; validation: multi-host fixture with current, stale and removed copies | Makes "is everyone on the same skill" answerable |
| `skills-targeting-users-and-agents` | planned | needs-contract | inline: applicability can name accounts, OIDC groups and agent principals in addition to environments, and every list and fetch rechecks current eligibility; validation: native and HTTP tests for each target kind, revocation and cross-Brain denial | Binds skills to environments, people and agents; depends on `agent-identity-principals` |
