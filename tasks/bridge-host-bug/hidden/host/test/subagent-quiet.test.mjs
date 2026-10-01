// Only a sub-agent's steps reach the panel. What it says (streamed or finished) and its plumbing
// calls stay out, and the top-level agent's answer still streams as before.

import assert from "node:assert/strict";
import { test } from "node:test";
import { newId, setup, subSteps } from "./subagent-helpers.mjs";

const ANSWER = "| Package | Version |\n| --- | --- |\n| requests | 2.32.3 |\n| httpx | 0.28.1 |";
const SUB_AGENT_SAID = ["Let me open a tab", "Opening the page", "as shown in the heading", "Found it in the page text"];

async function fanout(t) {
  const id = newId();
  await t.turn(id, "[fanout] compare");
  const ev = t.events(id);
  // The point of these tests is what else comes along once the steps are passed on.
  assert.ok(subSteps(ev).some((e) => e.toolUseId === "toolu_a2" && e.parent === "toolu_agent_a"), "the sub-agents' steps are reported");
  return ev;
}

test("a sub-agent's finished text is not shown as the agent's", async () => {
  const t = setup();
  try {
    const ev = await fanout(t);
    assert.deepEqual(ev.filter((e) => e.kind === "text").map((e) => e.text), [ANSWER]);
    const words = JSON.stringify(ev.filter((e) => e.kind === "text"));
    for (const said of SUB_AGENT_SAID) assert.ok(!words.includes(said), `"${said}" must not appear`);
  } finally {
    t.done();
  }
});

test("a sub-agent's streamed text is not shown either, and the agent's answer still streams whole", async () => {
  const t = setup();
  try {
    const ev = await fanout(t);
    const deltas = ev.filter((e) => e.kind === "text_delta");
    const full = ev.find((e) => e.kind === "text" && e.text === ANSWER);
    assert.ok(full, "the agent's answer arrived");
    assert.equal(deltas.map((d) => d.text).join(""), ANSWER);
    assert.ok(deltas.every((d) => d.messageId === full.messageId), "deltas and the final text share a message id");
  } finally {
    t.done();
  }
});

test("a sub-agent's ToolSearch calls are no more a step than the agent's own", async () => {
  const t = setup();
  try {
    const ev = await fanout(t);
    const tools = ev.filter((e) => e.kind === "tool_start" || e.kind === "tool_end");
    assert.ok(!tools.some((e) => e.name === "ToolSearch" || e.toolUseId === "toolu_a_ts"), "ToolSearch is not reported");
    assert.deepEqual(subSteps(ev).map((e) => e.toolUseId), ["toolu_a1", "toolu_b1", "toolu_a2", "toolu_b2", "toolu_b3"]);
  } finally {
    t.done();
  }
});

test("the prompt a sub-agent was started with is not echoed as a user message, and the turn ends once", async () => {
  const t = setup();
  try {
    const ev = await fanout(t);
    assert.deepEqual(ev.filter((e) => e.kind === "user").map((e) => e.text), ["[fanout] compare"]);
    assert.equal(ev.filter((e) => e.kind === "result").length, 1);
    assert.equal(ev.filter((e) => e.kind === "error").length, 0);
    assert.equal(ev.at(-1).kind, "status");
    assert.equal(ev.at(-1).status, "idle");
  } finally {
    t.done();
  }
});
