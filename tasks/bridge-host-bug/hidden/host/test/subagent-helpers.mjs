// Shared by the held-out sub-agent tests: host/chat.mjs against subagent-fake-claude.mjs, with HOME
// set to a temp dir per test (nothing outside it is read or written).

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createChat } from "../chat.mjs";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FAKE_CLAUDE = path.join(HERE, "subagent-fake-claude.mjs");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

let nextChat = 0;
export const newId = () => `00000000-0000-4000-8000-${String(++nextChat).padStart(12, "0")}`;

export function setup() {
  fs.chmodSync(FAKE_CLAUDE, 0o755); // the host only runs an executable file
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "fab-subagent-"));
  const sent = [];
  const chat = createChat({
    send: (m) => sent.push(m),
    log: () => {},
    home,
    env: { ...process.env, HOME: home, CLAUDE_BIN: FAKE_CLAUDE, CODEX_BIN: "/nonexistent/codex", SHELL: "/bin/false", CLAUDE_CONFIG_DIR: "", CODEX_HOME: "" },
  });
  const t = {
    home,
    sent,
    chat,
    events: (id) => sent.filter((m) => m.type === "chat.event" && m.chatId === id).map((m) => m.event),
    async until(what, fn, ms = 15000) {
      const end = Date.now() + ms;
      for (;;) {
        const v = fn();
        if (v) return v;
        if (Date.now() > end) throw new Error(`timed out waiting for ${what}\n${JSON.stringify(sent.slice(-12), null, 1).slice(0, 3000)}`);
        await sleep(15);
      }
    },
    results: (id) => t.events(id).filter((e) => e.kind === "result"),
    async turn(id, text) {
      const n = t.results(id).length;
      chat.handle({ type: "chat.send", chatId: id, engine: "claude", model: "claude-opus-5-5", effort: "high", text, attachments: [], context: { tabs: [] } });
      await t.until(`result ${n + 1} of ${id}`, () => t.results(id).length > n);
    },
    // chat.load: every chunk's items, once the last chunk is in.
    async load(fields) {
      const requestId = `r${sent.length}`;
      chat.handle({ type: "chat.load", requestId, ...fields });
      await t.until("the transcript", () => sent.find((m) => m.type === "chat.transcript" && m.requestId === requestId && m.done));
      return sent.filter((m) => m.type === "chat.transcript" && m.requestId === requestId).flatMap((m) => m.items);
    },
    done() {
      chat.shutdown();
      fs.rmSync(home, { recursive: true, force: true });
    },
  };
  return t;
}

// Summaries are compared with runs of whitespace collapsed, so a fix may keep or flatten line breaks.
export const flat = (s) => String(s ?? "").replace(/\s+/g, " ").trim();

// The text block Claude Code appends to a Task/Agent result; never part of the sub-agent's reply.
export const TRAILER_MARKS = ["agentId", "SendMessage", "<usage>", "total_tokens", "tool_uses", "duration_ms"];

// The steps a sub-agent took: tool_start events that name a parent.
export const subSteps = (events) => events.filter((e) => e.kind === "tool_start" && e.parent != null);
