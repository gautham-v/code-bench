// A sub-agent's steps finish in the panel: each tool_start it produced gets its tool_end.

import assert from "node:assert/strict";
import { test } from "node:test";
import { newId, setup } from "./subagent-helpers.mjs";

// ToolSearch is left out here; whether plumbing stays hidden is subagent-quiet.test.mjs's question.
const shown = (events, kind) => events.filter((e) => e.kind === kind && !String(e.toolUseId).endsWith("_ts"));

test("each of a sub-agent's steps ends, with the result summary a step of the agent's own would get", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    const ev = t.events(id);
    assert.ok(shown(ev, "tool_start").some((e) => e.toolUseId === "toolu_a1"), "the sub-agent's steps were reported");
    const ends = shown(ev, "tool_end").filter((e) => !String(e.toolUseId).startsWith("toolu_agent_"));
    assert.deepEqual(
      ends.map((e) => [e.toolUseId, e.ok, e.summary]),
      [
        ["toolu_a1", true, "Created tab 7 in the Claude tab group."],
        ["toolu_b1", true, "Created tab 8 in the Claude tab group."],
        ["toolu_b2", false, "TypeError: document.querySelector(...) is null"],
        ["toolu_a2", true, "Tab 7: https://pypi.org/project/requests/"],
        ["toolu_b3", true, ""],
      ],
    );
    // Nothing is left spinning: every step that started has ended, after it started, once.
    const order = ev.filter((e) => e.kind === "tool_start" || e.kind === "tool_end");
    for (const s of shown(ev, "tool_start")) {
      const mine = order.filter((e) => e.toolUseId === s.toolUseId).map((e) => e.kind);
      assert.deepEqual(mine, ["tool_start", "tool_end"], `${s.toolUseId} starts once and ends once`);
    }
    assert.ok(!JSON.stringify(ev).includes("stays on this computer"), "page text a sub-agent read is not passed on");
  } finally {
    t.done();
  }
});
