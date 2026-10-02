// Native OpenCode V2 adapter, checked against @opencode/plugin 2.0.21.
// Plugin.define is the identity function; a plain definition avoids runtime deps.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { readFile } from "node:fs/promises";
import { setTimeout as delay } from "node:timers/promises";

const executable = fileURLToPath(
  new URL("../bin/recollect-plugin", import.meta.url),
);

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
        return invoke({
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
    };
  },
};
