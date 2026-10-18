import { test } from "node:test";
import assert from "node:assert/strict";
import { chmod, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { join, resolve } from "node:path";
import { createNativeSkillReload } from "../../plugins/recollect/plugins/recollect-memory/opencode/index.mjs";

async function fixture(t) {
  const base = resolve(".cache", `opencode-native-unit-${randomUUID()}`);
  const workspace = join(base, "workspace");
  const dataRoot = join(base, "plugin");
  const device = randomUUID(), task = randomUUID(), brain = randomUUID(), scope = randomUUID(), id = randomUUID();
  const snapshot = join(dataRoot, "native-skills/catalogues", `${device}-${task}.json`);
  await mkdir(workspace, { recursive: true, mode: 0o700 });
  await mkdir(join(dataRoot, "native-skills/catalogues"), { recursive: true, mode: 0o700 });
  t.after(() => rm(base, { recursive: true, force: true }));
  const source = new Map();
  let registry = new Map(), transforms = [], reloads = 0, disposed = 0;
  const ctx = { skill: {
    async transform(callback) { transforms.push(callback); return { async dispose() { disposed++; } }; },
    async reload() {
      reloads++;
      registry = new Map(source);
      const editor = {
        list: () => [...registry.values()], get: (id) => registry.get(id),
        add: (skill) => registry.set(skill.id, skill), remove: (id) => registry.delete(id),
      };
      for (const callback of transforms) callback(editor);
    },
  } };
  const refresh = await createNativeSkillReload(ctx, { dataRoot });
  const skill = async (host = "opencode", version = 1, marker = "first body", skillID = id) => {
    const roots = { opencode: [".opencode", "opencode"], codex: [".agents", "codex"], claude_code: [".claude", "claude"] };
    const [root, prefix] = roots[host];
    const native_name = `recollect-${prefix}-${skillID.replaceAll("-", "")}`;
    const path = join(workspace, root, "skills", native_name, "SKILL.md");
    await mkdir(join(workspace, root, "skills", native_name), { recursive: true, mode: 0o700 });
    const provenance = { bundle_id: skillID, brain_id: brain, version, environment_id: null, bundle_environment_id: null };
    const content = `---\nname: ${JSON.stringify(native_name)}\ndescription: ${JSON.stringify('Use "quoted" technology guidance.')}\n---\n\nRecollect manager-published guidance, last fetched export. Provenance: \`${JSON.stringify(provenance)}\`.\n\nBefore use call current scoped skills.fetch.\n${marker}\n`;
    await writeFile(path, content, { mode: 0o600 });
    return { id: skillID, name: "technology-guidance", native_name, path, version, host, environment_id: null, brain_id: brain, persistent: false, content };
  };
  const publish = async (entries, owned_exports = entries.map(({ id, path, host, brain_id }) => ({ id, path, host, brain_id })), state = "placed") => {
    const body = { scope_id: scope, task_id: task, brain_id: brain, workspace, entries: entries.map(({ content, ...entry }) => entry), owned_exports };
    await writeFile(snapshot, JSON.stringify(body), { mode: 0o600 });
    return { recollectNativeSkillsSnapshot: { snapshot, workspace, state, changed: true, native_loaded: false } };
  };
  return { base, workspace, dataRoot, snapshot, id, brain, task, scope, source, skill, publish, refresh,
    get registry() { return registry; }, get transforms() { return transforms.length; },
    get reloads() { return reloads; }, get disposed() { return disposed; } };
}

test("native transform registers once, loads bounded generated skills and avoids no-op reload", async (t) => {
  const f = await fixture(t);
  const local = { id: "local", name: "local", path: join(f.workspace, ".opencode/skills/local/SKILL.md"), content: "Local skill" };
  f.source.set(local.id, local);
  const entry = await f.skill();
  const pointer = await f.publish([entry]);
  assert.equal((await f.refresh(pointer)).state, "refresh_requested");
  assert.equal(f.transforms, 1);
  assert.equal(f.reloads, 1);
  assert.equal(f.registry.get(entry.native_name).description, 'Use "quoted" technology guidance.');
  assert.equal(f.registry.get("local").content, "Local skill");
  assert.equal((await f.refresh(pointer)).changed, false);
  assert.equal(f.reloads, 1);
  assert.equal((await f.refresh({})).count, 1);
  assert.equal(f.reloads, 1);
  await f.refresh.dispose();
  assert.equal(f.disposed, 1);
});

test("replacement and explicit removal rebuild the real transform state", async (t) => {
  const f = await fixture(t);
  const original = await f.skill();
  f.source.set(original.native_name, { id: original.native_name, name: original.native_name, path: original.path, content: original.content });
  await f.refresh(await f.publish([original]));
  const updated = await f.skill("opencode", 2, "second body");
  const result = await f.refresh(await f.publish([updated]));
  assert.equal(result.native_loaded, false);
  assert.match(f.registry.get(updated.native_name).content, /second body/);
  assert.doesNotMatch(f.registry.get(updated.native_name).content, /first body/);
  await f.refresh(await f.publish([], [], "removed"));
  assert.equal(f.registry.has(updated.native_name), false);
  assert.equal(f.reloads, 3);
});

test("ownership changes deduplicate late compatibility exports while preserving foreign skills", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  await f.refresh(await f.publish([entry]));
  const compatibility = await f.skill("codex");
  const foreign = await f.skill("claude_code");
  f.source.set(compatibility.native_name, { id: compatibility.native_name, path: compatibility.path, content: compatibility.content });
  f.source.set(foreign.native_name, { id: foreign.native_name, path: foreign.path, content: foreign.content });
  // Only the exact receipt-verified compatibility path is owned by this snapshot.
  const owners = [entry, compatibility].map(({ id, path, host, brain_id }) => ({ id, path, host, brain_id }));
  assert.equal((await f.refresh(await f.publish([entry], owners))).changed, true);
  assert.equal(f.registry.has(compatibility.native_name), false);
  assert.equal(f.registry.has(foreign.native_name), true);
  assert.equal(f.registry.has(entry.native_name), true);
  assert.equal(f.reloads, 2);
});

test("transient absence, unavailable replies and malformed snapshots preserve valid registrations", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  await f.refresh(await f.publish([entry]));
  assert.equal((await f.refresh({ state: "unavailable" })).changed, false);
  assert.equal(f.registry.has(entry.native_name), true);
  await writeFile(f.snapshot, "{broken-json", { mode: 0o600 });
  assert.equal((await f.refresh({ snapshot: f.snapshot, workspace: f.workspace })).state, "unavailable");
  assert.equal((await f.refresh({})).count, 1);
  assert.equal(f.reloads, 1);
});

test("verified unavailable scope transition clears old guidance while busy peer scope stays registered", async (t) => {
  for (const state of ["unavailable", "scope_busy", "native_scope_busy"]) {
    const f = await fixture(t);
    const entry = await f.skill();
    f.source.set(entry.native_name, { id: entry.native_name, path: entry.path, content: entry.content });
    await f.refresh(await f.publish([entry]));
    const pointer = await f.publish([], [], state);
    const body = JSON.parse(await readFile(f.snapshot, "utf8"));
    body.scope_id = randomUUID();
    await writeFile(f.snapshot, JSON.stringify(body));
    const busy = state !== "unavailable";
    assert.equal((await f.refresh(pointer)).count, busy ? 1 : 0);
    assert.equal(f.registry.has(entry.native_name), busy);
  }
});

test("a trusted new scope with a missing resource withdraws the previous transform", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  f.source.set(entry.native_name, { id: entry.native_name, path: entry.path, content: entry.content });
  await f.refresh(await f.publish([entry]));
  const missing = await f.skill("opencode", 1, "New scope", randomUUID());
  const pointer = await f.publish([missing]);
  const body = JSON.parse(await readFile(f.snapshot, "utf8"));
  body.scope_id = randomUUID();
  await writeFile(f.snapshot, JSON.stringify(body));
  await rm(missing.path);
  const result = await f.refresh(pointer);
  assert.equal(result.state, "unavailable");
  assert.equal(result.count, 0);
  assert.equal(f.registry.has(entry.native_name), false);
});

test("absolute external paths, wrong identity and dynamic frontmatter never reach native state", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  const pointer = await f.publish([{ ...entry, path: join(f.base, "outside.md") }]);
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  assert.equal(f.reloads, 0);
  await f.publish([{ ...entry, brain_id: randomUUID() }]);
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  await f.publish([entry]);
  await writeFile(entry.path, entry.content.replace("description: ", "permission: allow\ndescription: "));
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  await writeFile(entry.path, entry.content + "\n!`do-not-execute`\n");
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  assert.equal(f.reloads, 0);
});

test("snapshot privacy, size bounds and symlink denial protect every filesystem entry", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  const pointer = await f.publish([entry]);
  await chmod(f.snapshot, 0o644);
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  await chmod(f.snapshot, 0o600);
  await writeFile(entry.path, "x".repeat(133 * 1024));
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  await rm(entry.path);
  const outside = join(f.base, "outside.md");
  await writeFile(outside, entry.content, { mode: 0o600 });
  await symlink(outside, entry.path);
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  assert.equal(await readFile(outside, "utf8"), entry.content);
  await rm(entry.path);
  await writeFile(entry.path, entry.content, { mode: 0o600 });
  const externalSnapshot = join(f.base, "external.json");
  await writeFile(externalSnapshot, await readFile(f.snapshot), { mode: 0o600 });
  await rm(f.snapshot);
  await symlink(externalSnapshot, f.snapshot);
  assert.equal((await f.refresh(pointer)).state, "unavailable");
  assert.equal(f.reloads, 0);
});

test("a foreign native ID collision is preserved rather than overwritten", async (t) => {
  const f = await fixture(t);
  const entry = await f.skill();
  const foreign = { id: entry.native_name, name: entry.native_name, path: join(f.workspace, ".opencode/skills/local/SKILL.md"), content: "User-authored content" };
  f.source.set(foreign.id, foreign);
  await f.refresh(await f.publish([entry]));
  assert.deepEqual(f.registry.get(foreign.id), foreign);
});
