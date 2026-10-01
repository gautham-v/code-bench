// A chat reopened from history shows the same reply on a Task/Agent step that it showed live.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { TRAILER_MARKS, flat, newId, setup } from "./subagent-helpers.mjs";

const TRAILER = "agentId: a1b2c3 (use SendMessage with to: 'a1b2c3' to continue this agent)\n<usage>total_tokens: 900\ntool_uses: 3\nduration_ms: 4100</usage>";
const ROWS = Array.from({ length: 40 }, (_, i) => `Desk ${i + 1}: $${400 + i * 7}, ${24 + (i % 9)} in deep`).join("\n");

// A Claude Code session file under the temp HOME, as a terminal session leaves it.
function sessionFile(home, project, id, entries) {
  const dir = path.join(home, ".claude/projects", project);
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `${id}.jsonl`);
  fs.writeFileSync(file, entries.map((e) => JSON.stringify({ sessionId: id, cwd: `/work/${project}`, ...e })).join("\n") + "\n");
  return file;
}
const call = (id, name, input) => ({ type: "assistant", uuid: `a-${id}`, message: { id: `m-${id}`, role: "assistant", model: "claude-opus-5-5", content: [{ type: "tool_use", id, name, input }] } });
const out = (id, content, isError = false) => ({ type: "user", uuid: `r-${id}`, message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content, ...(isError && { is_error: true }) }] } });
const endOf = (items, id) => items.find((e) => e.kind === "tool_end" && e.toolUseId === id);

test("chat.load of a terminal session: Task/Agent steps end with the sub-agent's reply", async () => {
  const t = setup();
  try {
    const S = "aaaaaaaa-0000-4000-8000-0000000000a1";
    const file = sessionFile(t.home, "proj-a", S, [
      { type: "user", uuid: "u1", message: { role: "user", content: "Compare the two packages" } },
      call("toolu_h1", "Agent", { description: "Read requests on PyPI", prompt: "Open the page", subagent_type: "general-purpose" }),
      call("toolu_h2", "Task", { description: "Read httpx on PyPI", prompt: "Open the page", subagent_type: "general-purpose" }),
      call("toolu_h0", "mcp__firefox__navigate", { url: "https://pypi.org/" }),
      out("toolu_h0", "Tab 4: https://pypi.org/\nTitle: PyPI"),
      out("toolu_h1", [{ type: "text", text: "requests 2.32.3" }, { type: "text", text: TRAILER }]),
      out("toolu_h2", "httpx 0.28.1\n(read from the page text)"),
      call("toolu_h3", "Read", { file_path: "/tmp/notes.txt" }),
      out("toolu_h3", "the file's private text"),
      { type: "assistant", uuid: "a-done", message: { id: "m-done", role: "assistant", model: "claude-opus-5-5", content: [{ type: "text", text: "Both are current." }] } },
    ]);
    const items = await t.load({ chatId: S, source: "terminal", path: file });
    assert.deepEqual(items.filter((e) => e.kind === "tool_start").map((e) => [e.toolUseId, e.summary]), [
      ["toolu_h1", "Read requests on PyPI"],
      ["toolu_h2", "Read httpx on PyPI"],
      ["toolu_h0", "Open pypi.org"],
      ["toolu_h3", "Read notes.txt"],
    ]);
    assert.equal(endOf(items, "toolu_h1").ok, true);
    assert.equal(flat(endOf(items, "toolu_h1").summary), "requests 2.32.3");
    assert.equal(flat(endOf(items, "toolu_h2").summary), "httpx 0.28.1 (read from the page text)");
    assert.equal(endOf(items, "toolu_h3").summary, "", "other tools' output still stays out");
    const all = JSON.stringify(items);
    for (const mark of TRAILER_MARKS) assert.ok(!all.includes(mark), `${mark} must not appear`);
  } finally {
    t.done();
  }
});

test("chat.load: a long reply is cut the same way, and a failed sub-agent keeps its error line", async () => {
  const t = setup();
  try {
    const S = "aaaaaaaa-0000-4000-8000-0000000000a2";
    const file = sessionFile(t.home, "proj-b", S, [
      { type: "user", uuid: "u1", message: { role: "user", content: "Compare forty desks" } },
      call("toolu_h5", "Agent", { description: "Read forty desks", prompt: "List them", subagent_type: "general-purpose" }),
      out("toolu_h5", [{ type: "text", text: ROWS }, { type: "text", text: TRAILER }]),
      call("toolu_h6", "Agent", { description: "Read the warranty page", prompt: "Read it", subagent_type: "general-purpose" }),
      out("toolu_h6", "Sub-agent hit an error\n    at step 2 of 3", true),
    ]);
    const items = await t.load({ chatId: S, source: "terminal", path: file });
    const long = endOf(items, "toolu_h5");
    assert.ok(long.summary.length <= 300, `summary is ${long.summary.length} characters`);
    assert.ok(flat(long.summary).startsWith("Desk 1: $400, 24 in deep Desk 2: $407, 25 in deep"));
    assert.ok(flat(long.summary).length >= 200);
    const fail = endOf(items, "toolu_h6");
    assert.equal(fail.ok, false);
    assert.ok(flat(fail.summary).startsWith("Sub-agent hit an error"), "the error is shown");
    assert.ok(!JSON.stringify(items).includes("agentId"));
  } finally {
    t.done();
  }
});

test("a panel chat reopened after a turn with sub-agents shows the replies it showed live", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    const live = Object.fromEntries(t.events(id).filter((e) => e.kind === "tool_end").map((e) => [e.toolUseId, flat(e.summary)]));
    const items = await t.load({ chatId: id });
    assert.equal(flat(endOf(items, "toolu_agent_a").summary), "requests 2.32.3");
    assert.equal(flat(endOf(items, "toolu_agent_b").summary), "httpx 0.28.1 (read from the page text)");
    assert.equal(flat(endOf(items, "toolu_agent_a").summary), live.toolu_agent_a, "the same as it was live");
    assert.equal(flat(endOf(items, "toolu_agent_b").summary), live.toolu_agent_b, "the same as it was live");
  } finally {
    t.done();
  }
});
