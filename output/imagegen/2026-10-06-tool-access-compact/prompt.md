# Compact Tool access concept — 2026-10-06

Generated with the built-in image-generation tool. Concept only; no application
code, grants, environment configuration or runtime state changed.

The attached screenshot was inspected as a visual reference. This proposed
layout keeps independent permissions and shows inherited administrative rights
with locked icons. A future implementation must preserve canonical direct/group
grant behavior and distinguish permitted editing from inherited effective rights.

## Final prompt

Use case: ui-mockup.
Asset type: One polished, implementable desktop UI concept for Recollect's Tool access page, showing a cleaner inline tool-group editor.
Primary request: Redesign the current oversized editing card and cluttered People permissions, preserving the liked MCP-add workflow. Make a sharp flat UI screenshot, not a photograph or a mood board.
Canvas: landscape 1600 by 1050, generously readable, straight-on. Light cream canvas #f7f4ec, off-white cards #fffdf8, ink #101b2a, sage #3d6b59, fine warm grey borders. Refined serif ONLY for the page heading and modest section labels; compact sans-serif inputs and names. No dark mode, shadows, gradients, giant headings, oversized form fields, bottom assistant bar, explanatory paragraphs, sidebar, or decorative content.
Composition: Main page area centered with a bounded width about 1160px. At top a restrained page heading "Connections". Tabs "Connections", "Tool access", "Runners"; Tool access selected with a sage underline. Next a compact filter dropdown "All environments", 240px wide on the left, button "+ Create tool group" aligned right. Do not stretch the filter across the entire page.
Main card: One tool group open in inline editing, a compact pale sage header only 68px tall. A small ink square with an outlined book icon, then a modest 18px sans-serif group-name input showing "Demo web research", width 320px and height 34px, not huge and not spanning the card. On the SAME header row: an "Environment" label with a 170px dropdown "Brain-wide", then a small green toggle and text "Enabled" at the right. These controls should feel as compact as normal toolbar inputs.
Below header, show a very small muted "Add description" text button, not a full-width empty form field.
Card body: balanced two columns separated by whitespace and one subtle vertical hairline. Left section heading "MCPs" with a tiny "1 / 20" count. One neat connection row with cube icon and name "Exa", small secondary "2 tools", an unobtrusive remove x at far right. A thin separator below. Preserve familiar Add MCP workflow: compact label "Add MCP" and ordinary connection dropdown "Choose a connection", about 380px wide, 34px high.
Right column: heading "People" and small "+ Add person" action. Show ONLY ONE permission-icon set per person. Above the icons, three tiny aligned column labels "Use", "Manage", "Share". Each row height 56px: small circular avatar, username on the left, one unobtrusive access-source chip immediately beside the username, and exactly three 28px rounded icon buttons on the right under the labels. Use icons: outlined play triangle; Manage icons: outlined sliders; Share icons: outlined connected-three-nodes. All icon sets align consistently.
People data precisely: row 1 avatar "D", username "demo", small neutral source chip "Direct"; Use has pale GREEN background/green icon, Manage and Share have pale RED background/red icons. Row 2 avatar "O", username "owner", tiny "(you)" and neutral source chip "Admin"; all three permissions GREEN. On owner's Manage and Share icons, add a very small lock mark to show these permissions come from administrative access. Do not show a second Direct/Effective icon set or duplicate rows. No opened inheritance audit, roles paragraphs, Groups: None, repeated effective/direct headings, or "Your access" footer. Sources can be inspected by clicking the small Direct/Admin chip; depict the clean closed state with no tooltip open. Keeping inherited permissions locked must look intentional.
Footer: thin top divider with small right-aligned "Cancel" outline button and "Save changes" ink button, normal 34px height. No large empty area between people and footer. Main editing card overall about 420px high.
Below with 20px gap, a second smaller card in normal read mode: a restrained header with book icon and name "test", tiny "Brain-wide" pill, subtle green "Enabled" pill and small pencil "Edit". Its compact body previews "Context7 · proof" with cube icon and "2 tools" on left and an "owner" avatar/name with the same three green permission icons on right. Show real concise controls and rows, no random instructional copy.
Text fidelity: crisp legible labels exactly as quoted. Permissions must remain independent. This is a proposed design, not a dashboard reporting new live state. Deliver just one coherent beautiful UI screenshot, no side-by-side variants or annotations.
