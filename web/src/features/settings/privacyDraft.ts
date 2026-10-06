import type { components } from "../../api-schema";

export type PrivacySnapshot = {
  capture: components["schemas"]["CaptureSettings"];
  retention: components["schemas"]["RetentionSettings"];
  documents: boolean;
  repository: boolean;
};
export type PrivacyDraft = {
  capture: components["schemas"]["CapturePolicy"];
  retention: components["schemas"]["RetentionPolicy"];
  documents: boolean;
  repository: boolean;
};
export type PrivacyDomain = keyof PrivacyDraft;
export const privacyLabels: Record<PrivacyDomain, string> = {
  capture: "Capture",
  retention: "Retention",
  documents: "Document storage",
  repository: "Repository storage",
};
export function privacyDraft(snapshot: PrivacySnapshot): PrivacyDraft {
  return structuredClone({
    capture: snapshot.capture.policy,
    retention: snapshot.retention.policy,
    documents: snapshot.documents,
    repository: snapshot.repository,
  });
}
export function privacyChanges(
  snapshot: PrivacySnapshot,
  draft: PrivacyDraft,
): PrivacyDomain[] {
  const saved = privacyDraft(snapshot);
  // Apply duration changes last: shortening them can expire existing content.
  return (["capture", "documents", "repository", "retention"] as const).filter(
    (key) => JSON.stringify(saved[key]) !== JSON.stringify(draft[key]),
  );
}
export function privacyIsCurrent(
  base: PrivacySnapshot,
  current: PrivacySnapshot,
) {
  return (
    base.capture.change_id === current.capture.change_id &&
    base.retention.change_id === current.retention.change_id &&
    base.documents === current.documents &&
    base.repository === current.repository
  );
}
export type PrivacyCommands = {
  capture: (
    base: string,
    policy: PrivacyDraft["capture"],
  ) => Promise<PrivacySnapshot["capture"]>;
  retention: (
    base: string,
    policy: PrivacyDraft["retention"],
  ) => Promise<PrivacySnapshot["retention"]>;
  documents: (value: boolean) => Promise<boolean>;
  repository: (value: boolean) => Promise<boolean>;
};
export function validatePrivacyDraft(draft: PrivacyDraft) {
  const { capture, retention } = draft;
  const durations = [
    retention.raw_session_days, retention.tool_output_days,
    retention.document_days, retention.support_excerpt_days,
    retention.repository_days, retention.claim_days, retention.audit_days,
  ];
  if (durations.some((days) => days != null && (!Number.isInteger(days) || days < 1 || days > 3650)) ||
      !Number.isInteger(retention.backup_days) || retention.backup_days < 1 || retention.backup_days > 365)
    throw new Error("Use 1–3,650 days for retention and 1–365 days for backups.");
  if (!Number.isInteger(capture.max_event_bytes) || capture.max_event_bytes < 1024 || capture.max_event_bytes > 65536 ||
      !capture.kinds.length || new Set(capture.kinds).size !== capture.kinds.length ||
      capture.kinds.some((kind) => !["prompt", "reply", "tool_result", "lifecycle"].includes(kind)))
    throw new Error("Choose at least one capture kind and a 1–64 KiB event limit.");
  if ([capture.excluded_tools, capture.excluded_content].some((rules) => rules.length > 20 ||
      rules.some((rule) => !rule.trim() || new TextEncoder().encode(rule).length > 200 || /[\u0000-\u001f\u007f-\u009f]/.test(rule))))
    throw new Error("Use up to 20 exclusions per field, each at most 200 bytes without control characters.");
}
/** Keep receipts for successful domains if a later canonical command fails.
 * Retrying from the acknowledged snapshot sends only the unfinished changes.
 */
export async function savePrivacyDraft(
  base: PrivacySnapshot,
  current: PrivacySnapshot,
  draft: PrivacyDraft,
  commands: PrivacyCommands,
  acknowledge: (snapshot: PrivacySnapshot, domain: PrivacyDomain) => void,
): Promise<PrivacySnapshot> {
  if (!privacyIsCurrent(base, current))
    throw new Error("Privacy settings changed elsewhere. Cancel and reopen to edit them.");
  validatePrivacyDraft(draft);
  let saved = base;
  for (const domain of privacyChanges(base, draft)) {
    if (domain === "capture")
      saved = {
        ...saved,
        capture: await commands.capture(saved.capture.change_id, draft.capture),
      };
    else if (domain === "retention")
      saved = {
        ...saved,
        retention: await commands.retention(
          saved.retention.change_id,
          draft.retention,
        ),
      };
    else saved = { ...saved, [domain]: await commands[domain](draft[domain]) };
    acknowledge(saved, domain);
  }
  return saved;
}
