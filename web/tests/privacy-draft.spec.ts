import { test, expect } from "@playwright/test";
import {
  privacyDraft, privacyChanges, savePrivacyDraft,
  type PrivacyCommands, type PrivacySnapshot,
} from "../src/features/settings/privacyDraft";

const snapshot = (): PrivacySnapshot => ({
  capture: { brain_id: "brain", change_id: "capture-1", policy: {
    enabled: true, kinds: ["prompt", "reply", "tool_result", "lifecycle"],
    managed_tools: true, max_event_bytes: 32768,
    excluded_tools: ["private.*"], excluded_content: ["private phrase"],
  } },
  retention: { brain_id: "brain", change_id: "retention-1", policy: {
    raw_session_days: 30, tool_output_days: 30, document_days: null,
    support_excerpt_days: null, repository_days: null, claim_days: null,
    audit_days: 365, backup_days: 7, allow_support_excerpts: true,
  } },
  documents: true, repository: false,
});
function commands(base: PrivacySnapshot, calls: string[]): PrivacyCommands {
  return {
    capture: async (revision, policy) => {
      calls.push("capture:" + revision);
      return { ...base.capture, change_id: "capture-2", policy };
    },
    retention: async (revision, policy) => {
      calls.push("retention:" + revision);
      return { ...base.retention, change_id: "retention-2", policy };
    },
    documents: async (value) => { calls.push("documents"); return value; },
    repository: async (value) => { calls.push("repository"); return value; },
  };
}
test("partial save retains acknowledgements and retry skips completed domains", async () => {
  const original = snapshot();
  const draft = privacyDraft(original);
  draft.capture.max_event_bytes = 65536;
  draft.documents = false;
  draft.retention.audit_days = 180;
  let acknowledged = original;
  const calls: string[] = [];
  const first = commands(original, calls);
  first.documents = async () => { calls.push("documents:failed"); throw new Error("Storage unavailable"); };
  await expect(savePrivacyDraft(original, original, draft, first, (value) => { acknowledged = value; })).rejects.toThrow("Storage unavailable");
  expect(acknowledged.capture.change_id).toBe("capture-2");
  expect(acknowledged.capture.policy.excluded_tools).toEqual(["private.*"]);
  expect(acknowledged.capture.policy.excluded_content).toEqual(["private phrase"]);
  expect(acknowledged.retention).toEqual(original.retention);
  expect(privacyChanges(acknowledged, draft)).toEqual(["documents", "retention"]);
  const saved = await savePrivacyDraft(acknowledged, acknowledged, draft, commands(acknowledged, calls), (value) => { acknowledged = value; });
  expect(calls).toEqual(["capture:capture-1", "documents:failed", "documents", "retention:retention-1"]);
  expect(saved.documents).toBe(false);
  expect(saved.retention.policy.audit_days).toBe(180);
  expect(original.capture.policy.max_event_bytes).toBe(32768);
});
test("fresh revision or storage drift prevents every write", async () => {
  const base = snapshot();
  const draft = privacyDraft(base);
  draft.documents = false;
  for (const current of [
    { ...base, capture: { ...base.capture, change_id: "capture-new" } },
    { ...base, retention: { ...base.retention, change_id: "retention-new" } },
    { ...base, documents: false }, { ...base, repository: true },
  ]) {
    const calls: string[] = [];
    await expect(savePrivacyDraft(base, current, draft, commands(base, calls), () => {})).rejects.toThrow("changed elsewhere");
    expect(calls).toEqual([]);
  }
});
test("unchanged forms and invalid duration drafts perform no commands", async () => {
  const base = snapshot();
  const draft = privacyDraft(base);
  const calls: string[] = [];
  await savePrivacyDraft(base, base, draft, commands(base, calls), () => {});
  expect(calls).toEqual([]);
  draft.capture.max_event_bytes = 65536;
  draft.retention.audit_days = 0;
  await expect(savePrivacyDraft(base, base, draft, commands(base, calls), () => {})).rejects.toThrow("days");
  expect(calls).toEqual([]);
});
test("accepted odd-byte capture limits survive unrelated retention changes", async () => {
  const base = snapshot();
  base.capture.policy.max_event_bytes = 16385;
  const draft = privacyDraft(base);
  draft.retention.backup_days = 14;
  const calls: string[] = [];
  const saved = await savePrivacyDraft(base, base, draft, commands(base, calls), () => {});
  expect(calls).toEqual(["retention:retention-1"]);
  expect(saved.capture.policy.max_event_bytes).toBe(16385);
});
