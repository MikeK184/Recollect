# Strict independent visual review — round 2

**Result: changes required in Privacy, AI, and setup-card spacing.** Actual captures were independently inspected at 1586 × 992, 1440 × 900, and 1920 × 1080 against the six final approved images. No frontend source was edited by the reviewer.

## Corrected

- Connections at reference size: selected row begins about y351 versus target y350; search is 438 px; private-network card begins y821 versus target819; inspector ends y938, matching the reference. Truthful rounded call-state pills, primary configuration chips, secondary disclosures, and the header-aligned inspector now establish the intended composition.
- Config dialog matches the main-pane position of the reference and has fully visible footer buttons. The numbered editor uses the intended readable syntax styling; authentication, server summary, name, and credentials remain arranged horizontally. At 1440 × 900, the lower name field requires body scrolling while the footer stays visible.
- Global connector page retains the actually selected authorized Brain navigation. Catalogue cells, separate card footers, import/paste rail, and connected numbered steps follow the target. The real single-card inventory is preserved. At 1440 × 900, the page flows vertically rather than horizontally clipping.
- Agents now has the plus icon, larger host marks, correctly spaced dates/tab labels, and full-width memory-copy action. Roster names, avatar, pills, and row hierarchy follow the target.
- AI model names and icons are readable again, model badges no longer take most of the row, and all three More options rows exist in the reference-size capture.

## Still required

| Surface | Remaining measured difference | Required correction |
| --- | --- | --- |
| Privacy edit | Capture is about 253 px versus reference217; Evidence is about223 px versus164. Repository begins around y950 rather than y859 and is cut by the reference-size viewport. | Compact the extra disclosure/auxiliary policy grouping and section heights while keeping all policy classes and edit access. |
| Privacy view and other sections | Read-only retention values still appear as disabled number/unit boxes. | Keep stable row locations, but display plain readable values outside the relevant edit state as in the reference. |
| AI | More options begins about y775 versus757, and its card bottom is about1011 versus953; the final row/border is cut at992h. | Compact its heading gap and row/card spacing so the full card fits. |
| AI | Enabled is still small unframed text in the memory strip. Allowed-content indicators are faint standalone checks rather than filled sage squares with white checks. | Match the reference pill and checked-square treatments without making them interactive in view state. |
| Agents | Setup/memory card bottoms are about y991 versus971. | Reduce the remaining footer/padding gap by about20 px. |
| Connections | Primary row order is Target / Runs on / Scope, whereas target uses Target / Scope / Runs on. | Match reference ordering; keep additional metadata in its existing disclosure. |
| Config feedback | The transport and masking sentence visually touch. | Retain explicit whitespace between them. |

## Limits and remaining proof

These are independent pixel observations of parent-produced runtime captures. Parent functional tests/DOM measurements are separate evidence. Lower modal fields still need explicit body-bottom captures at 1440 × 900 and 1920 × 1080 for edit and approved-connector setup, including schema/enabled controls; lower AI/library/Privacy actions need page-bottom captures at 1440 × 900. Natural page scrolling at shorter height is acceptable when controls remain reachable and fully rendered. No external connection/model checks, policy saves, real credentials, or inventory mutation were performed for this review.

The subsequently supplied `after/02-edit-bottom-1440.jpg` was independently inspected: the complete settings editor, schema disclosure, enabled checkbox, explanatory text, Close and Save buttons are visibly present inside the modal. Parent reports Save bottom813 < viewport900; this is supplied DOM evidence rather than an independent measurement.

Parent's isolated functional test also identified a real Capture master-switch click interception by its section header overlay. This remains a blocking interaction failure in round 2 and is scheduled for correction before round 3 acceptance. Pixel review alone does not establish working controls.
