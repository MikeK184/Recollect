# Branded README workflows

Observed: 2026-10-06
Confidence: verified

## Sources and Method

The user approved the agent and private-runner layouts, requested the actual
logo in every image, and asked for a closer match to current Tool access cards
with clearer service execution and environment context. This documentation-only
small fix follows the existing desktop/catalogue/private-runner contracts and
the [original identity](../../web/public/brand/README.md).

The original `recollect-logo.svg` and `recollect-symbol.svg` were rasterized
unchanged using the installed sharp dependency for reference inputs. All edits
to generated images used built-in `image_gen.imagegen`. The agent/private
images received targeted logo edits; the environment illustration used the
approved private-runner composition and actual
[Tool access badge screenshot](../../output/tool-access-header-badges-2026-10-06/read.png).
[Prompts and provenance](../../output/imagegen/2026-10-06-readme-branded/prompts.json)
record exact inputs, generation prompts and final source/destination paths.

## Observations

The three final workspace assets are:

- [Agent memory v3](../assets/recollect-agent-workflow-v3.png): original logo,
  preserving the approved roster, Brain card, approved tools and title.
- [Environment v3](../assets/recollect-environment-access-v3.png): agent request
  and result arrows, original Recollect mark in Access & dispatch, Development
  group with separate environment/MCP badges, Brain-wide Exa/Context7 and
  Recollect-service placement badges; one compact Use-only reader row.
- [Private runner v3](../assets/recollect-private-runner-v3.png): original logo
  and Recollect-card mark, preserving the paired Ubuntu card and outbound,
  request and result directions.

Visual inspection matched the original mark's interrupted arcs, inner loop,
diamond and coloured dots, with the approved cream/ink/sage styling. These are
generated renditions of the supplied identity, not vector masters. All PNGs
verify as RGB at 1672×941. The local GitHub-style preview loaded all eight images
without horizontal overflow at the default 1280px viewport. Its
[environment-section capture](../../output/imagegen/2026-10-06-readme-branded/readme-preview.png)
shows readable cards/arrows at README size. All 39 local README links/anchors,
governance lint/32 tests and whitespace checks passed.

## Translation and Limits

The README distinguishes central dispatch from remotely hosted HTTP MCP servers:
the agent calls Recollect, which checks access and makes the approved request.
Environment scope governs availability independently of executor placement.
Examples remain labelled product illustrations, not exact screenshots or current
Connected/runtime proof. The v3 assets supersede the
[preceding concept imagery](readme-concept-imagery-2026-10-06.md) in the README;
v2 assets remain historical.

No runtime/product code, permissions, deployment or provider tool calls changed.
The temporary preview tab/server were closed without changing viewport settings.
Version N/A; the continuing user-authorized local commit is recorded in the
containing history. No push.
