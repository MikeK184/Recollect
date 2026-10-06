# Independent final management UI review — 5 October 2026

**Historical review — superseded.** The user rejected this implementation's visual
match and explicitly required closer fidelity to the six final references. The
acceptance below reflects the earlier, insufficiently strict review standard and
must not be treated as current acceptance. Current findings are in
[vision-fidelity/target-checklist.md](vision-fidelity/target-checklist.md).

Status: **visual acceptance passed for all six surfaces at 1440 × 900 and
1920 × 1080**. No remaining visual blocker was found in the final captures.
This is visual acceptance within the evidence boundaries below, not a claim of
independently repeated final interaction tests or external connectivity.

## Evidence and independence

The reviewer compared the original live baseline, all six approved image concepts,
the first implementation and the corrected local runtime. The original independent
browser pass exercised navigation, modal methods, safe config parsing, error
recovery, filters, host switching, Privacy editing and cancel without saving shared
data or calling external servers/providers. Its findings are in
[after-first.md](after-first.md), with retained captures in [after/](after/).

During the final pass the reviewer's browser session became unavailable: browser
inventory/selection worked, but tab acquisition and listing timed out after fresh
session resets. The primary agent's browser connection remained usable. Therefore
the final comparison uses fresh, unmodified JPEG captures of the real local UI
collected by the primary agent through the documented browser/CDP screenshot API
and independently inspected by the reviewer. Final interactions and DOM measurements
are primary-agent evidence, not independently repeated interaction proof.

Thirty-two final captures were independently inspected in [after-final/](after-final/).
They cover six main surfaces at both sizes, tool previews, URL/config methods,
approved/edit form tops and true ends, filtered-empty library results, Privacy
display/edit, settled AI diagnostics and malformed-config error/recovery. Concept names, sample counts,
icons and connector cards are illustrative; the review evaluates layout and clarity
against the approved direction while preserving actual authorized inventory and
governing policy classes. Configured credentials or historical observations do not
prove a current server or provider call succeeded.

## Before, vision and final comparison

| Surface | Original baseline | Approved direction and final result |
| --- | --- | --- |
| Connections | Broad single card, scattered Test/Edit/Pause, Details/history and Advanced runner controls. | Selected rows show target, scope/placement, concise date and Inspect. A nearby Overview/Tools/Activity inspector holds visible main actions; Tool access and Runners are peer navigation. [1440](after-final/01-connections-1440.jpg), [1920](after-final/01-connections-1920.jpg). |
| Add/edit connection | Anonymous URL flow separated from a long advanced wizard/editor; no direct secret/config setup. | One entry offers URL, config and approved definition methods; authentication is explicit, a file action and parsed server/transport feedback are visible. Long form bodies scroll while primary/cancel controls stay exposed. [Config 1440](after-final/02-add-config-parsed-1440.jpg), [edit end 1440](after-final/02-edit-bottom-1440.jpg), [approved end 1920](after-final/02-add-approved-bottom-1920.jpg). |
| Global Connectors | No independent installation library page. | A global library presents the actual approved definition, compact search/state filters and contextual import/review guidance. Filtered-empty results offer Clear filters rather than suggesting duplicate setup. [1440](after-final/03-connectors-1440.jpg), [1920](after-final/03-connectors-1920.jpg). |
| Privacy | Editing replaced a summary with a visually different inset form. | Same labeled retention rows persist during edit; fields replace the values in place, with warning and save/cancel below. A short status badge, separate policy cards and secondary exclusions preserve the governing distinctions. [View 1920](after-final/04-privacy-view-1920.jpg), [edit 1920](after-final/04-privacy-edit-1920.jpg). |
| AI permissions | Flat prose summary followed by expanded diagnostics, coverage and batch history. | Purposes and permitted content share a compact primary card; coverage prominently shows represented and blocked/failed counts with attention feedback. Provider/model details and explicit diagnostics are separate. All seven content classes now fit within the laptop viewport. [1440](after-final/05-ai-1440.jpg), [1920](after-final/05-ai-1920.jpg), [diagnostics](after-final/05-ai-diagnostics-1920.jpg). |
| Agents | Compressed roster above a largely empty page, with troubleshooting links. | Clear owner/credential/observed-use roster, filled Connect action, host setup and memory-read cards. Long prompts sit behind disclosures and copy actions are accessible. [1440](after-final/06-agents-1440.jpg), [1920](after-final/06-agents-1920.jpg). |

The light cream/ink/sage palette, serif headings, Recollect mark and existing icon
family remain coherent across all surfaces. No visible horizontal overflow,
button/label collisions or hidden primary form action remained in the captures.
Vertical scrolling remains appropriate for preserved Privacy classes and secondary
setup/diagnostic details. The real one-definition catalogue is intentionally sparser
than the illustrative six-card concept; no fabricated inventory is expected.

## Closure of first-pass findings

| Finding | Final evidence / disposition |
| --- | --- |
| UI-01 — low inspector | Corrected: main inspector actions are visible at 900px height. Primary-agent DOM measurement placed their bottom at 820.75px. |
| UI-02 — clipped/absent footer | Corrected: parsed config and approved/edit footers remain exposed at both heights. True end captures show schema/enabled controls above the footer, rather than merely stopping midway through the form. |
| UI-03 — growing sidebar gap | Corrected: global links and Brain picker stay in a compact flow at both heights; extra space is reserved before the account footer. |
| UI-04 — incomplete row summary | Corrected: target URL, Brain/environment placement, concise historical date and Inspect affordance are readable. |
| UI-05 — missing library context | Corrected: actual catalogue now has a contextual import/review side panel. |
| UI-06 — confusing filtered-empty result | Corrected: no-match text points to search/approval filters and offers Clear filters. |
| UI-07 — stretching Privacy badge / density | Corrected: badge stays a short pill; rows are compact and maintain display/edit structure without combining distinct policy classes. |
| UI-08 — content below fold / weak attention hierarchy | Corrected after one further independent finding: purposes use compact inline descriptions, all content classes fit within 900px, blocked/failed count is prominent, dimensions are in diagnostics. |
| UI-09 — long agent prompts / misleading subtitle | Corrected: full prompts are disclosed on demand, Connect is primary, wording accurately describes observed use and credential status. |
| UI-10 — invalid approved draft enables Save | Corrected in pixels: Save remains disabled with the definition selected but required name/target blank at both sizes. Final validation behavior is primary-agent fixture evidence. |
| UI-11 — unbounded tool descriptions | Corrected: each approved tool has a bounded preview and explicit Full description disclosure. |
| UI-12 — missing parsed feedback / file action | Corrected in pixels: server count, transport and Choose config file are visible. Malformed JSON displays a clear Configuration could not be parsed panel with supported-format/size guidance, and replacing it with safe valid config produces the parsed summary without an error. [Error](after-final/02-add-config-error-1440.jpg), [recovery](after-final/02-add-config-recovered-1440.jpg). Final interaction sequence is primary-agent evidence; the first-pass direct independent error/recovery also passed. A line gutter remains optional polish, not an acceptance requirement. |

## Supplied final geometry and interaction evidence

These are primary-agent observations, kept separate from the independent pixel review:

- At 1440 × 900, parsed-config footer buttons occupy y825–865. Approved/edit
  final checkbox occupies y735–755 above the footer y808–881 after normal keyboard
  Tab navigation scrolls the actual form to its end.
- Privacy's Raw sessions label at 1920 × 1080 retains identical document coordinates
  in view/edit: x309, y736.484375. Locator clicks changed scroll offset in some paired
  screenshots, explaining the viewport-coordinate difference. Cancel was used
  without saving; isolated geometry/cancel proof also passed.
- Settled AI captures show 412 represented and 27 blocked or failed; these are point
  observations, not immutable acceptance values. At 1440, the last allowed-content
  chip ends around y857.8, within the 900px viewport.
- Final capture interactions used only navigation, filters, opening/cancelling forms,
  host/disclosure state, malformed JSON and harmless config containing an example URL and
  `${DOCS_TOKEN}` placeholder. No credential value, provider check, server inspection
  or save was submitted. The capture tab was closed and viewport override reset.

## Validation boundaries

No final-state provider/model check, connection test, credential entry, policy save,
definition approval or other inventory mutation is part of this independent review.
Role loss, stale revision, duplicate/replay, credential provider failure and retry
cases are isolated primary-agent fixture evidence, not independently reproduced here.
External connectivity is not claimed. Final direct browser interaction replay was
skipped because of the review-session connection failure; it does not invalidate
the independent final pixel comparison, but must remain explicit in closeout.
