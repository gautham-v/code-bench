// A Task/Agent step ends with what the sub-agent replied (at most 300 characters), without the
// block Claude Code appends to it. Other tools' output still never leaves the host.

import assert from "node:assert/strict";
import { test } from "node:test";
import { TRAILER_MARKS, flat, newId, setup } from "./subagent-helpers.mjs";

const endOf = (events, id) => events.find((e) => e.kind === "tool_end" && e.toolUseId === id);

test("the step ends with the sub-agent's reply, whether the tool is called Agent or Task", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    const ev = t.events(id);
    const a = endOf(ev, "toolu_agent_a");
    const b = endOf(ev, "toolu_agent_b");
    assert.equal(a.ok, true);
    assert.equal(flat(a.summary), "requests 2.32.3");
    assert.equal(b.ok, true);
    assert.equal(flat(b.summary), "httpx 0.28.1 (read from the page text)", "the whole reply, not its first line");
  } finally {
    t.done();
  }
});

test("Claude Code's agentId/usage block is not part of the reply", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[fanout] compare");
    await t.turn(id, "[replies] compare");
    const ev = t.events(id);
    assert.ok(flat(endOf(ev, "toolu_agent_a").summary).startsWith("requests 2.32.3"), "the reply is there");
    const all = JSON.stringify(ev);
    for (const mark of TRAILER_MARKS) assert.ok(!all.includes(mark), `${mark} must not appear`);
  } finally {
    t.done();
  }
});

test("a long reply is cut to 300 characters at most", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[replies] compare");
    const long = endOf(t.events(id), "toolu_long");
    assert.equal(long.ok, true);
    assert.ok(long.summary.length <= 300, `summary is ${long.summary.length} characters`);
    assert.ok(flat(long.summary).startsWith("Desk 1: $400, 24 in deep Desk 2: $407, 25 in deep Desk 3: $414, 26 in deep"), "it starts with the start of the reply");
    assert.ok(flat(long.summary).length >= 200, "and keeps most of what fits");
  } finally {
    t.done();
  }
});

test("a failed sub-agent's step still ends as a failure with its error line; other tools' output stays put", async () => {
  const t = setup();
  try {
    const id = newId();
    await t.turn(id, "[replies] compare");
    const ev = t.events(id);
    assert.ok(flat(endOf(ev, "toolu_long").summary).startsWith("Desk 1"), "replies are passed on");
    const fail = endOf(ev, "toolu_fail");
    assert.equal(fail.ok, false);
    assert.ok(flat(fail.summary).startsWith("Sub-agent hit an error"), "the error is shown");
    assert.equal(endOf(ev, "toolu_page").summary, "");
    assert.equal(endOf(ev, "toolu_read").summary, "");
    assert.ok(!JSON.stringify(ev).includes("private text"));
  } finally {
    t.done();
  }
});
