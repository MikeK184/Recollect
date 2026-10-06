# Strict independent visual review — round 1

**Result: changes required.** The six final approved image variants were inspected directly, followed by actual runtime captures at 1586 × 992. This review rejects the remaining differences below; it does not reuse the earlier broad acceptance.

## Evidence and limits

- Targets: `01-connections.png`, `02-add-connection.png`, `03-global-connectors-v2.png`, `04-privacy.png`, `05-ai-permissions-v2.png`, `06-agents.png` in `output/imagegen/2026-10-05-management-concepts-01a10b36/`.
- Actual: the corresponding files in `vision-fidelity/round-1/`, independently inspected as original pixels. Runtime captures were produced by the parent using documented browser APIs.
- Approximate coordinates below are pixel observations, not independent DOM measurements. Parent is responsible for interaction fixtures and runtime measurements.
- Some initial captures were made during entrance transitions. The parent subsequently supplied settled replacements, including loaded Privacy view, and these were re-inspected; transient opacity and the superseded loading capture are not reported as CSS defects.
- Different inventory sizes, real names, configured/observed states, real host prompts, the existing product mark, and truthful local credential-provider wording are not visual defects. No fake connections, approval queues, rows, calls, permissions, or secrets are requested.

## Required corrections

| Surface | Severity | Observed deviation | Target correction |
| --- | --- | --- | --- |
| Connections | Major | Selected row starts about y385 versus y350; the extra loaded-count spacing adds about 35 px. Private-network card begins y855 versus y819. Search is about 509 px wide versus 438 px. | Compact the count/toolbar gap and reserve the search width shown in the reference; preserve the count accessibly. |
| Connections inspector | Major | Test status starts about y588 versus y450, Tool access y780 versus y628, and the rail exceeds the bottom of the reference. Connector, credentials, description, and duplicated tool-access metadata occupy the primary configuration area. | Keep Target, Scope, and Runs on as the three primary rows; fold secondary metadata into a disclosure. Use reference chip treatment for execution placement. |
| Connections row status | Moderate | Configured is unframed small text. | Use a rounded truthful status pill with the recorded-state indication; a previously recorded successful call must not imply a current connected session. |
| Add config and edit | Major | Footer buttons are visibly cut at the modal's bottom around y970. Edit has lower content and required enabled/schema controls not represented in the initial top capture. | Keep the footer fully visible, independently measure its button bounds, and prove all lower fields reachable through the body scroll at each desktop height. |
| Add config | Moderate | Icons touch method labels, and numbered editor text is about 12 px versus reference 14 px. | Add explicit inline icon spacing and raise code text to the reference size without changing the safe imported configuration or provider semantics. |
| Global connectors | Moderate | Import rail ends around y962 versus y938; body text is slightly smaller. The loaded global page currently omits the selected authorized Brain section. | Retain the user's explicitly approved last selected authorized Brain context, without first-Brain guessing or changing global API ownership; fit the rail close to the reference bottom. |
| Privacy | Major | In the reference-matched raw-retention edit state, Capture is about 271 px high versus 217 px; its header-to-row gap pushes Evidence about 52 px down. The loaded view also confirms Evidence is about 255 px high versus the reference 164 px, with an oversized header gap and two auxiliary policy rows; Repository begins about y951 and is below the reference viewport composition. Capture/Evidence titles are about 14 px instead of 20–22 px. | Compact section header padding and rows, group secondary policies while retaining each distinct policy, and restore the stronger reference titles. |
| Privacy | Moderate | Row labels/helpers are about 12 px instead of 14 px. Evidence rows show disabled number/unit controls across unrelated classes instead of the reference's plain read values. Enabled capture switches are beige even in loaded view, whereas the reference retains sage checked state. | Preserve stable row geometry with readable labels and values; confine editable controls to the relevant edit state where practical, and retain sage checked-state styling for read-only switches. Recheck loaded view, edit, and Cancel with preserved values. |
| AI permissions | Major | Automatic memory strip is about 89 px versus 66 px. Its Enabled text lacks a rounded pill and the description wraps despite the wide primary card. | Widen the text column, content-size the Enabled pill, and match the compact strip. |
| AI side rail | Major | Coverage is about 347 px versus 300 px; Models 283 px versus 195 px. More options begins y901 instead of y757, leaving Policy history and Recent batches below the viewport. | Move operational counts/prose into existing diagnostics, compact model rows, make status pills content-sized, and show all three More options actions. Preserve complete model names and detail access. |
| AI typography | Moderate | Model/provider icons are about 16–18 px versus 28–32 px. Purpose/model labels are about 14 px versus 16 px; descriptions about 12 px versus 14 px. | Restore the reference icon and text hierarchy while correcting the excess card spacing. |
| Agents | Major | Memory-copy button is about 226 px wide rather than filling its card; setup and memory card borders extend below 992 px. | Make the primary memory-copy action full width and reduce setup footer spacing so both card bottoms fit near the reference y971. |
| Agents | Moderate | Missing plus on Connect agent; host glyphs are small; host tab icons touch labels; Observed and the date have no text gap. Secondary text is about 16 px versus 18 px. | Match the reference icon scale, button icon, inline spacing, and readable secondary text; retain the truthful command/prompt. |

## What now follows the reference

- Connections header and adjacent inspector, sage selected row, stronger serif type, dark cube tile, and private-network card structure.
- Add connection's broad dialog, method strip, format tabs, numbered syntax-colored editor, parsed feedback, horizontal server summary, and authentication columns.
- Global connector catalogue cell sizing and height, distinct card footer, import/paste actions, numbered review steps, and reusable-definition explanation. A single actual card is not evidence of a density defect.
- Privacy retention-first layout with same-row number/unit controls and a warning/action line.
- AI primary/provider structure, explicit processing rows, allowed-content grid, separate rail cards, and truthful blocked counts.
- Agents' owner/avatar group, weighted roster names, credential pills, setup host strip, actual connection prompt, and separate memory check.

## Next proof

Recapture loaded and animation-settled screens after corrections, then inspect all six at 1586 × 992, 1440 × 900, and 1920 × 1080. Include long approved/edit modal bottoms, Privacy view/edit/Cancel, AI More options, and setup card bottoms. Parent interaction proofs remain separate from independent pixel acceptance.
