"use strict";

// Show tabs lasts while the client that asked for it is connected. Once it disconnects, its
// sessions are ordinary ones again.
const test = require("node:test");
const assert = require("node:assert/strict");
const { load, SHOW } = require("./show-tabs-env.js");

// The MCP server of the same session, connected again: the host gives it a new client id.
const AGAIN = { id: 3, name: "claude-code" };
const AGAIN_SHOWN = { id: 4, name: "claude-code" };

// A session that was shown, whose client then disconnected and came back without showTabs. The
// user is on their own tab 1.
async function ended() {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const tab = await env.newTab("demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [tab], "shown while its client is connected");
  await env.disconnect(SHOW);
  await env.connect(AGAIN);
  await env.browser.userSelects(1);
  return { ...env, tab };
}

test("after the client disconnects, the session's new tabs open in the background", async () => {
  const env = await ended();
  const next = await env.newTab("demo", AGAIN);
  assert.equal(env.browser.groupOf(next), env.browser.groupOf(env.tab), "the same session, the same group");
  assert.deepEqual(env.browser.activeIn(10), [1]);
  env.browser.userFocusesWindow(20);
  await env.newTab("demo", AGAIN);
  assert.equal(env.browser.focusedWindow(), 20, "no window is raised");
});

test("after the client disconnects, calls no longer select the session's tab", async () => {
  const env = await ended();
  await env.call("find", { tabId: env.tab, query: "x" }, "demo", AGAIN);
  await env.call("computer", { action: "left_click", tabId: env.tab, coordinate: [10, 10] }, "demo", AGAIN);
  assert.deepEqual(env.browser.activeIn(10), [1]);
});

for (const together of [false, true]) {
  test(`after the client disconnects, focus is handed back for a tab the session's page opens (${together ? "activated before onCreated is done" : "created, then activated"})`, async () => {
    const env = await ended();
    await env.call("find", { tabId: env.tab, query: "x" }, "demo", AGAIN);
    const opened = await env.browser.pageOpensTab(env.tab, { together });
    assert.deepEqual(env.browser.activeIn(10), [1]);
    assert.equal(env.browser.groupOf(opened), env.browser.groupOf(env.tab));
  });
}

// The disconnect itself ends it: nobody has connected or called since.
for (const together of [false, true]) {
  test(`with its client gone and nobody reconnected, focus is handed back for a tab the session's page opens (${together ? "activated before onCreated is done" : "created, then activated"})`, async () => {
    const env = await load();
    await env.connect(SHOW, { showTabs: true });
    const tab = await env.newTab("demo", SHOW);
    assert.deepEqual(env.browser.activeIn(10), [tab], "shown while its client is connected");
    await env.disconnect(SHOW);
    await env.browser.userSelects(1);
    const opened = await env.browser.pageOpensTab(tab, { together });
    assert.deepEqual(env.browser.activeIn(10), [1]);
    assert.equal(env.browser.groupOf(opened), env.browser.groupOf(tab));
  });
}

test("a client that connects with showTabs again shows the session again", async () => {
  const env = await ended();
  await env.disconnect(AGAIN);
  await env.connect(AGAIN_SHOWN, { showTabs: true });
  const next = await env.newTab("demo", AGAIN_SHOWN);
  assert.deepEqual(env.browser.activeIn(10), [next]);
});
