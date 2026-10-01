// A sub-agent's tool calls reach the panel as steps under the Task/Agent call that started it.

import assert from "node:assert/strict";
import { test } from "node:test";
import { newId, setup } from "./subagent-helpers.mjs";

// ToolSearch is left out here; whether plumbing stays hidden is subagent-quiet.test.mjs's question.
const shown = (events, kind) => events.filter((e) => e.kind === kind && !String(e.toolUseId).endsWith("_ts"));

test("two sub-agents running at once: each call is a tool_start naming its own Task/Agent step as parent", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    const starts = shown(t.events(id), "tool_start");
    assert.deepEqual(
      starts.map((e) => [e.toolUseId, e.name, e.parent ?? null]),
      [
        ["toolu_agent_a", "Agent", null],
        ["toolu_agent_b", "Task", null],
        ["toolu_a1", "mcp__firefox__tabs_create_mcp", "toolu_agent_a"],
        ["toolu_b1", "mcp__firefox__tabs_create_mcp", "toolu_agent_b"],
        ["toolu_a2", "mcp__firefox__navigate", "toolu_agent_a"],
        ["toolu_b2", "mcp__firefox__javascript_tool", "toolu_agent_b"],
        ["toolu_b3", "mcp__firefox__get_page_text", "toolu_agent_b"],
      ],
    );
  } finally {
    t.done();
  }
});

test("a sub-agent's steps carry the same summary and tabId the agent's own steps do", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    const ev = t.events(id);
    const byId = Object.fromEntries(shown(ev, "tool_start").map((e) => [e.toolUseId, e]));
    for (const step of ["toolu_a1", "toolu_a2", "toolu_b2", "toolu_b3"]) assert.ok(byId[step], `${step} was reported`);
    assert.equal(byId.toolu_agent_a.summary, "Read requests on PyPI");
    assert.equal(byId.toolu_a1.summary, "Open a new tab");
    assert.equal(byId.toolu_a2.summary, "Open pypi.org/project/requests/");
    assert.equal(byId.toolu_b2.summary, "Run a script on the page");
    assert.equal(byId.toolu_b3.summary, "Read the page text");
    // The steps card names the site from the latest step's tab.
    assert.ok(byId.toolu_a1.tabId == null, "a call that names no tab has no tabId");
    assert.equal(byId.toolu_a2.tabId, 7);
    assert.equal(byId.toolu_b2.tabId, 8);
    assert.equal(byId.toolu_b3.tabId, 8);
    const all = JSON.stringify(ev);
    for (const secret of ["querySelector('h1')", "?q=1"]) assert.ok(!all.includes(secret), `${secret} must not appear`);
  } finally {
    t.done();
  }
});
