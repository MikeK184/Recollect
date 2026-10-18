// Native OpenCode V2 adapter, checked against @opencode/plugin 2.0.21.
// Plugin.define is the identity function; a plain definition avoids runtime deps.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { lstat, open, readFile, realpath } from "node:fs/promises";
import { dirname, isAbsolute, join, parse, relative, resolve } from "node:path";
import { homedir } from "node:os";
import { setTimeout as delay } from "node:timers/promises";

const executable = fileURLToPath(
  new URL("../bin/recollect-plugin", import.meta.url),
);

const uuidPattern = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const compactUUID = /^[a-f0-9]{32}$/;

function pluginDataRoot() {
  return process.env.RECOLLECT_PLUGIN_DATA ??
    (process.platform === "darwin"
      ? join(homedir(), "Library/Application Support/Recollect/plugin")
      : join(process.env.XDG_DATA_HOME ?? join(homedir(), ".local/share"), "recollect/plugin"));
}

function checkedAbsolute(value) {
  if (typeof value !== "string" || !isAbsolute(value) || value !== resolve(value) || /[\x00-\x1f]/.test(value)) {
    throw new Error("invalid_native_path");
  }
  return value;
}

async function checkedDirectory(value) {
  const path = checkedAbsolute(value);
  let current = parse(path).root;
  for (const segment of relative(current, path).split(/[\\/]/).filter(Boolean)) {
    current = join(current, segment);
    const info = await lstat(current);
    if (!info.isDirectory() || info.isSymbolicLink()) throw new Error("unsafe_native_directory");
  }
  if (await realpath(path) !== path) throw new Error("noncanonical_native_directory");
  return path;
}

async function checkedText(path, limit, privateFile = false) {
  checkedAbsolute(path);
  await checkedDirectory(dirname(path));
  const info = await lstat(path);
  if (!info.isFile() || info.isSymbolicLink() || info.size > limit || (info.mode & 0o111) || (privateFile && (info.mode & 0o077))) {
    throw new Error("unsafe_native_file");
  }
  // Bound reads even when an owned file grows after stat. Symlink ancestors are
  // checked before opening and the final open descriptor is compared to lstat.
  const file = await open(path, "r");
  try {
    const opened = await file.stat();
    if (!opened.isFile() || opened.dev !== info.dev || opened.ino !== info.ino || opened.size > limit) throw new Error("unsafe_native_file");
    const bytes = Buffer.alloc(limit + 1);
    const { bytesRead } = await file.read(bytes, 0, bytes.length, 0);
    if (bytesRead > limit) throw new Error("native_file_too_large");
    const content = new TextDecoder("utf-8", { fatal: true }).decode(bytes.subarray(0, bytesRead));
    if (content.includes("!`")) throw new Error("invalid_native_dynamic_context");
    return content;
  } finally {
    await file.close();
  }
}

function provenance(content) {
  const match = content.match(/^Recollect manager-published guidance, last fetched export\. Provenance: `(\{[^\n]+\})`\.$/m);
  if (!match) throw new Error("native_provenance_missing");
  return JSON.parse(match[1]);
}

function nativeDescription(content, name) {
  const header = content.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/)?.[1];
  if (!header) throw new Error("invalid_native_frontmatter");
  const lines = header.split(/\r?\n/);
  // The Rust exporter emits exactly these benign JSON-quoted YAML fields.
  if (lines.length !== 2 || !lines[0].startsWith("name: ") || !lines[1].startsWith("description: ")) throw new Error("invalid_native_frontmatter");
  const actualName = JSON.parse(lines[0].slice(6));
  const description = JSON.parse(lines[1].slice(13));
  if (actualName !== name || typeof description !== "string" || !description.trim() || [...description].length > 1024) throw new Error("invalid_native_frontmatter");
  return description;
}

/** One native transform, updated from bounded private runtime snapshots. Calling
 * this helper requests a registry rebuild; it never claims model consumption. */
export async function createNativeSkillReload(ctx, { dataRoot = pluginDataRoot() } = {}) {
  checkedAbsolute(dataRoot);
  let current = new Map();
  let currentWorkspace;
  let currentBrain;
  let currentScope;
  let known = new Map();
  let signature;
  let tail = Promise.resolve();
  let needsReload = false;
  let registration;
  let registered = false;
  const transform = (editor) => {
    for (const candidate of editor.list()) {
      if (typeof candidate.path !== "string" || !known.has(candidate.path) || typeof candidate.content !== "string") continue;
      try {
        const source = provenance(candidate.content);
        const owner = known.get(candidate.path);
        if (source.bundle_id !== owner.id || source.brain_id !== owner.brain_id) continue;
      } catch { continue; }
      // Deduplicate only recognized generated exports. An unrelated local skill,
      // even one with a similar name, remains the host's native entry.
      editor.remove(candidate.id);
    }
    for (const skill of current.values()) {
      const candidate = editor.get(skill.id);
      if (!candidate) editor.add(skill);
      // A foreign entry with the same ID is never silently overwritten.
    }
  };

  const refresh = (reply) => {
    const work = tail.then(async () => {
      const pointer = reply?.recollectNativeSkillsSnapshot ?? reply;
      if (!pointer || typeof pointer.snapshot !== "string") return { state: "unchanged", changed: false, count: current.size, native_loaded: false };
      // The registry is location-wide. A conflicting task uses scoped MCP while
      // the active peer's native catalog keeps its live references.
      if (["scope_busy", "native_scope_busy"].includes(pointer.state)) return { state: "scope_busy", changed: false, count: current.size, native_loaded: false };
      let boundary;
      try {
        const workspace = await checkedDirectory(pointer.workspace);
        if (workspace === parse(workspace).root || workspace === homedir()) throw new Error("invalid_native_workspace");
        const catalogue = join(dataRoot, "native-skills/catalogues");
        const snapshotPath = checkedAbsolute(pointer.snapshot);
        const basename = relative(catalogue, snapshotPath);
        const fileMatch = basename.match(/^([a-f0-9-]{36})-([a-f0-9-]{36})\.json$/);
        if (!fileMatch || !uuidPattern.test(fileMatch[1]) || !uuidPattern.test(fileMatch[2])) throw new Error("native_snapshot_path_mismatch");
        const snapshot = JSON.parse(await checkedText(snapshotPath, 1024 * 1024, true));
        if (snapshot.workspace !== workspace || !uuidPattern.test(snapshot.scope_id) || !uuidPattern.test(snapshot.task_id) || !uuidPattern.test(snapshot.brain_id) || snapshot.task_id !== fileMatch[2] || !Array.isArray(snapshot.entries) || snapshot.entries.length > 64) throw new Error("invalid_native_snapshot");
        if (currentScope && (currentScope !== snapshot.scope_id || currentBrain !== snapshot.brain_id || currentWorkspace !== workspace)) {
          boundary = { workspace, brain: snapshot.brain_id, scope: snapshot.scope_id };
        }
        const next = new Map();
        const nextKnown = new Map(known);
        const ownedExports = snapshot.owned_exports ?? [];
        if (!Array.isArray(ownedExports) || ownedExports.length > 192) throw new Error("invalid_native_snapshot_ownership");
        for (const owner of ownedExports) {
          const host = { codex: [".agents", "codex"], claude_code: [".claude", "claude"], opencode: [".opencode", "opencode"] }[owner.host];
          if (!host || !uuidPattern.test(owner.id) || owner.brain_id !== snapshot.brain_id || owner.path !== join(workspace, host[0], "skills", `recollect-${host[1]}-${owner.id.replaceAll("-", "")}`, "SKILL.md")) throw new Error("invalid_native_snapshot_ownership");
          nextKnown.set(owner.path, owner);
        }
        for (const entry of snapshot.entries) {
          if (!uuidPattern.test(entry.id) || entry.brain_id !== snapshot.brain_id || entry.host !== "opencode" || !Number.isInteger(entry.version) || entry.version <= 0 || entry.environment_id != null && !uuidPattern.test(entry.environment_id)) throw new Error("invalid_native_snapshot_entry");
          const compact = entry.id.replaceAll("-", "");
          if (!compactUUID.test(compact)) throw new Error("invalid_native_snapshot_entry");
          const name = `recollect-opencode-${compact}`;
          const path = join(workspace, ".opencode/skills", name, "SKILL.md");
          if (entry.native_name !== name || entry.path !== path || next.has(name)) throw new Error("native_snapshot_path_mismatch");
          const content = await checkedText(path, 132 * 1024);
          const source = provenance(content);
          if (source.bundle_id !== entry.id || source.brain_id !== snapshot.brain_id || source.version !== entry.version || source.bundle_environment_id !== (entry.environment_id ?? null)) throw new Error("native_snapshot_provenance_mismatch");
          const description = nativeDescription(content, name);
          next.set(name, { id: name, name, description, path, content, autoinvoke: true });
          nextKnown.set(path, { id: entry.id, brain_id: snapshot.brain_id, host: "opencode", path });
        }
        if (nextKnown.size > 4096) throw new Error("native_ownership_limit");
        const nextSignature = JSON.stringify([workspace, snapshot.brain_id, snapshot.scope_id,
          [...next.values()].sort((a, b) => a.path.localeCompare(b.path)),
          ownedExports.toSorted((a, b) => a.path.localeCompare(b.path))]);
        const changed = signature !== nextSignature;
        // Only a complete valid snapshot changes the transform. A transport,
        // permission, malformed or partial-file failure preserves prior state.
        if (changed) {
          current = next;
          currentWorkspace = workspace;
          currentBrain = snapshot.brain_id;
          currentScope = snapshot.scope_id;
          known = nextKnown;
          signature = nextSignature;
          needsReload = true;
        }
        if (!registered) {
          // OpenCode 2.0.21 installs its filesystem source after plugin setup.
          // Register once on the first actual context snapshot, after native
          // catalog initialization, so this deduplication runs after that source.
          await ctx.skill.list?.();
          registration = await ctx.skill.transform(transform);
          registered = true;
        }
        if (needsReload) {
          await ctx.skill.reload();
          needsReload = false;
        }
        return { state: changed ? "refresh_requested" : "unchanged", changed, count: current.size, brain_id: currentBrain, scope_id: currentScope, native_loaded: false };
      } catch {
        if (boundary) {
          // A verified scope transition withdraws the previous transform even
          // if a new resource is missing or the subsequent download was partial.
          current = new Map();
          currentWorkspace = boundary.workspace;
          currentBrain = boundary.brain;
          currentScope = boundary.scope;
          signature = undefined;
          needsReload = true;
          if (registered) {
            try { await ctx.skill.reload(); needsReload = false; } catch {}
          }
        }
        return { state: "unavailable", changed: Boolean(boundary), count: current.size, native_loaded: false };
      }
    });
    tail = work.catch(() => {});
    return work;
  };
  refresh.dispose = async () => { await tail; await registration?.dispose?.(); };
  return refresh;
}

export function invoke(payload, program = executable) {
  return new Promise((resolve) => {
    const input = JSON.stringify(payload);
    if (Buffer.byteLength(input) > 256 * 1024) return resolve({});
    const child = spawn(program, ["hook", "opencode"], {
      stdio: ["pipe", "pipe", "ignore"],
    });
    let output = "";
    const timer = setTimeout(() => child.kill(), 9500);
    child.on("error", () => {
      clearTimeout(timer);
      resolve({});
    });
    child.stdin.on("error", () => {});
    child.stdout.on("data", (chunk) => {
      output += chunk;
      if (Buffer.byteLength(output) > 32 * 1024) child.kill();
    });
    child.on("close", () => {
      clearTimeout(timer);
      try {
        resolve(JSON.parse(output));
      } catch {
        resolve({});
      }
    });
    child.stdin.end(input);
  });
}

export default {
  id: "recollect-memory",
  async setup(ctx) {
    const reloadNativeSkills = await createNativeSkillReload(ctx);
    const controller = new AbortController();
    // Per-session queues preserve event order without serializing different sessions.
    const tails = new Map();
    const sessions = new Map();
    const pendingReplies = new Map();
    const waiting = new Map();
    const flushing = new Map();
    let closing = false;
    const serial = (id, work) => {
      const result = (tails.get(id) ?? Promise.resolve())
        .then(work)
        .catch(() => ({}));
      tails.set(id, result);
      void result.finally(() => {
        if (tails.get(id) === result) tails.delete(id);
      });
      return result;
    };
    const emit = (id, event, fields = {}) =>
      serial(id, async () => {
        const session =
          event === "SessionEnd" || event === "CoverageGap" || event === "Stop"
            ? sessions.get(id)
            : await ctx.session.get({ sessionID: id });
        if (!session) return {};
        sessions.set(id, session);
        const parent = session.parentID
          ? (sessions.get(session.parentID) ??
            (await ctx.session.get({ sessionID: session.parentID })))
          : null;
        const reply = await invoke({
          hook_event_name: event,
          session_id: id,
          ...(parent
            ? {
                parent_session_id: parent.id,
                parent_cwd: parent.location.directory,
              }
            : {}),
          cwd: session.location.directory,
          host_version: ctx.app.version,
          ...fields,
        });
        await reloadNativeSkills(reply);
        return reply;
      });
    const captureReply = async (id, messages) => {
      const pending = pendingReplies.get(id);
      if (!pending) return;
      const completed = messages.filter(
        (message) =>
          message.type === "assistant" &&
          message.time?.completed != null &&
          !pending.seen.has(message.id),
      );
      if (!completed.length) return;
      if (completed.length !== 1) {
        pendingReplies.delete(id);
        await emit(id, "CoverageGap");
        return;
      }
      const message = completed[0];
      const text = message.content
        .filter((part) => part.type === "text")
        .map((part) => part.text)
        .join("\n");
      // Only the completed visible message from this observed model request.
      // Historical messages, provider state and reasoning are never captured.
      if (text) {
        const result = await emit(id, "Stop", {
          prompt_id: message.id,
          cwd: pending.cwd,
          last_assistant_message: text,
          recollect_capture_binding: pending.binding,
        });
        if (!result.recollectCapturedEvent) return;
      }
      if (pendingReplies.get(id) === pending) pendingReplies.delete(id);
    };
    const flushReply = async (id, messages) => {
      const work = (flushing.get(id) ?? Promise.resolve()).then(() =>
        captureReply(id, messages),
      );
      flushing.set(id, work);
      try {
        await work;
      } finally {
        if (flushing.get(id) === work) flushing.delete(id);
      }
    };
    const waitForReply = (id) => {
      if (waiting.has(id)) return;
      const work = ctx.session
        .wait({ sessionID: id }, { signal: controller.signal })
        .then(async () => {
          if (!closing)
            await flushReply(id, await ctx.session.context({ sessionID: id }));
        })
        .catch(async () => {
          if (!closing) await emit(id, "CoverageGap");
        })
        .finally(() => {
          if (waiting.get(id) === work) waiting.delete(id);
        });
      waiting.set(id, work);
    };
    const skills = await Promise.all(
      ["recollect-memory", "recollect-connect"].map(async (name) => {
        const path = fileURLToPath(
          new URL(`../skills/${name}/SKILL.md`, import.meta.url),
        );
        const content = await readFile(path, "utf8");
        const description =
          content.match(/^description: (.+)$/m)?.[1] ?? "Recollect memory";
        return { id: name, name, description, path, content, autoinvoke: true };
      }),
    );
    await ctx.skill.transform((editor) => {
      for (const skill of skills) if (!editor.get(skill.id)) editor.add(skill);
    });
    await ctx.mcp.transform((editor) => {
      const existing = editor.get("recollect");
      if (
        existing &&
        JSON.stringify(existing.command) !== JSON.stringify([executable, "mcp"])
      ) {
        throw new Error(
          "Recollect already has an MCP entry. Migrate that entry before enabling the packaged plugin.",
        );
      }
      // Native tools are verified on 2.0.21. Its initial Code Mode catalogue can
      // omit plugin-added MCP tools even after the server reports connected.
      editor.set("recollect", {
        type: "local",
        command: [executable, "mcp"],
        codemode: false,
      });
    });
    await ctx.session.hook("context", async (event) => {
      // Read the documented session API, not provider-formatted messages (which
      // may quote or wrap text) or transcript files. Only admitted user text.
      const messages = await ctx.session.context({
        sessionID: event.sessionID,
      });
      await flushReply(event.sessionID, messages);
      const latest = [...messages]
        .reverse()
        .find((message) => message.type === "user");
      const query = latest?.text;
      // Context contains admitted user messages. The earlier prompt hook runs
      // before admission, so capturing there could retain a cancelled draft.
      const reply = await emit(
        event.sessionID,
        latest?.id ? "UserPromptSubmit" : "ContextRequest",
        {
          prompt: query,
          ...(latest?.id ? { prompt_id: latest.id } : {}),
        },
      );
      const text = reply.hookSpecificOutput?.additionalContext;
      if (text) event.system.push({ type: "text", text });
      if (pendingReplies.has(event.sessionID))
        await emit(event.sessionID, "CoverageGap");
      pendingReplies.delete(event.sessionID);
      if (reply.recollectCaptureBinding) {
        // The assistant ID does not exist until after this hook. Keep only the
        // immutable operation reference and observed message IDs, never a copy
        // of the transcript. Completion is read through the documented API.
        pendingReplies.set(event.sessionID, {
          binding: reply.recollectCaptureBinding,
          cwd: sessions.get(event.sessionID).location.directory,
          seen: new Set(messages.map((message) => message.id)),
        });
        waitForReply(event.sessionID);
      }
    });
    await ctx.session.hook("compaction", async (event) => {
      await emit(event.sessionID, "PreCompact");
      // The next context hook retrieves again under current scope and authority.
    });
    await ctx.tool.hook("execute.before", async (event) => {
      const pending = pendingReplies.get(event.sessionID);
      await emit(event.sessionID, "StepStart", {
        prompt_id: event.messageID,
        ...(pending && !pending.seen.has(event.messageID)
          ? { recollect_capture_binding: pending.binding }
          : {}),
      });
      await emit(event.sessionID, "PreToolUse", {
        prompt_id: event.messageID,
        tool_use_id: event.id,
        tool_name: event.tool,
        tool_input: event.input,
      });
    });
    await ctx.tool.hook("execute.after", async (event) => {
      await emit(
        event.sessionID,
        event.status === "error" ? "PostToolUseFailure" : "PostToolUse",
        {
          prompt_id: event.messageID,
          tool_use_id: event.id,
          tool_name: event.tool,
          tool_input: event.input,
          ...(event.status === "error"
            ? { error: event.error }
            : { tool_response: event.result }),
        },
      );
    });
    const subscription = async () => {
      try {
        for await (const event of ctx.event.subscribe({
          signal: controller.signal,
        })) {
          try {
            const data = event.data;
            // The server stream can include other project locations. Only
            // sessions observed by this plugin's native hooks belong to it.
            if (!data?.sessionID || !sessions.has(data.sessionID)) continue;
            if (event.type === "session.step.ended") {
              const messages = await ctx.session.context({
                sessionID: data.sessionID,
              });
              await flushReply(data.sessionID, messages);
            }
            if (event.type === "session.execution.interrupted")
              await emit(data.sessionID, "Interrupt");
            if (event.type === "session.deleted") {
              await emit(data.sessionID, "SessionEnd");
              sessions.delete(data.sessionID);
              pendingReplies.delete(data.sessionID);
            }
          } catch {
            // One inaccessible/deleted session must not stop capture for all other sessions.
          }
        }
      } catch {
        // Resume the stream below. Reply completion also has the native wait/API path.
      }
    };
    void (async () => {
      while (!closing) {
        await subscription();
        if (closing) break;
        await Promise.allSettled(
          [...sessions.keys()].map((id) => emit(id, "CoverageGap")),
        );
        try {
          await delay(1000, undefined, { signal: controller.signal });
        } catch {
          break;
        }
      }
    })();
    return async () => {
      closing = true;
      controller.abort();
      await Promise.allSettled([...flushing.values()]);
      for (const sessionID of sessions.keys()) {
        try {
          await flushReply(sessionID, await ctx.session.context({ sessionID }));
        } catch {
          await emit(sessionID, "CoverageGap");
        }
      }
      await Promise.allSettled(
        [...sessions.keys()].map((id) => emit(id, "SessionEnd")),
      );
      await reloadNativeSkills.dispose();
    };
  },
};
