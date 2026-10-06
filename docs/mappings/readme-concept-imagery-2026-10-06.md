# README concept imagery

Observed: 2026-10-06
Confidence: verified

## Sources and Method

The user rejected the preceding SVG workflow artwork and requested generated
images matching Recollect's styling and approved concepts. This documentation-only
small fix belongs to the repository-governance epic. The
[desktop visual language](../contracts/desktop-experience.md),
[catalogue contract](../contracts/mcp-catalogue-and-profiles.md) and
[private-runner contract](../contracts/mcp-vault-and-private-runners.md) remain
the product authority.

The built-in `image_gen.imagegen` tool used the approved
[Private Runners](../../output/imagegen/2026-10-06-private-runners-relationships/view-v2.png)
and [Tool access](../../output/imagegen/2026-10-06-tool-access-compact/tool-access-v2.png)
concepts as visual references. The complete
[prompt set and source/destination provenance](../../output/imagegen/2026-10-06-readme-concepts/prompts.json)
records all three generations and the private-runner correction. That correction
removed unrequested coding-host labels and replaced an invented glyph with a
simple outline icon.

## Observations

The README embeds three generated PNG illustrations:

- [Agent memory](../assets/recollect-agent-workflow-v2.png): supported coding
  hosts, plugin/direct MCP, shared Brain knowledge and separate approved tools.
- [Environment permissions](../assets/recollect-environment-access-v2.png):
  Brain-wide Exa, a Development group and independent Use/Manage/Share icons.
- [Private execution](../assets/recollect-private-runner-v2.png): a coding agent
  calls Recollect; the paired Ubuntu runner connects outbound and executes
  approved filesystem tools within the private network.

Visual inspection confirmed cream/ink/sage colours, serif headings, compact
card headers, scope/count badges, fine dividers and readable identifiers. Pillow
verified all three RGB PNG files: 1672×941, 1672×940 and 1672×941 respectively.
The local GitHub-style README preview loaded all eight images at the default
1280px browser viewport with no horizontal overflow. The
[preview capture](../../output/imagegen/2026-10-06-readme-concepts/readme-preview.png)
records the new agent illustration in the README. All 39 local README links and
anchors passed. Governance lint, 32 checker tests and whitespace checks passed.

## Translation and Limits

These are product illustrations using example data, explicitly labelled in the
README. They are not exact UI screenshots or evidence of current Connected
status. They supersede the README artwork in the
[preceding workflow record](readme-runner-workflows-2026-10-06.md); the old SVGs
and earlier proof remain historical artifacts.

Only documentation and images changed. No application/runtime validation,
provider tool execution, deployment or push was performed. The temporary preview
tab/server were closed; no viewport override was applied. Version N/A; the
user-authorized follow-up local commit is identified by the containing history.
