# Generated image review

Status: proposed
Reviewed: 2026-09-26

All 12 selected images were visually inspected after built-in image generation.
They depict the nine Brain routes and three global routes with sample data.
The initial Devices variant accidentally retained the Brain Manage group; a
targeted image-tool edit removed it before saving the selected image here.

The images communicate layout and visual direction. Generated text, badges,
dates, icons and spacing are illustrative; the written plan and design tokens
govern implementation. In particular, Agents setup cards mean supported hosts,
not proof that hosts are currently active; connection URLs/credential references
are invented examples, not configuration instructions; and Settings does not
establish that answering is currently supported or enabled.

Final files: [screen inventory](README.md#screen-inventory).
Prompts: [generation set](prompts.json).

## Devices correction prompt

Use case: ui-mockup. Edit only the left sidebar of this existing Recollect desktop Devices mockup. Remove the entire MANAGE group and its three entries Connections, Activity, Settings including icons and the divider above MANAGE. Leave that area empty warm cream sidebar. The GLOBAL sidebar must contain exactly Brains, Team, Devices, with Devices active, plus the existing Recollect wordmark and bottom Mike identity. Preserve every pixel of the main Devices content, typography, colors, page dimensions, pairing request, right instructions, footer and topbar as closely as possible. Do not add any new content or navigation.
