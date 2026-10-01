"use strict";

// Show tabs is per client: while a show-tabs client is connected, every other client's sessions
// keep working in background tabs, and focus is still handed back for the tabs their pages open.
const test = require("node:test");
const assert = require("node:assert/strict");
const { load, SHOW, PLAIN } = require("./show-tabs-env.js");

// SHOW has a tab in front; PLAIN (same client name, no showTabs) has a session of its own.
async function both(plainFlags) {
  const env = await load();
  await env.connect(PLAIN, plainFlags);
  await env.connect(SHOW, { showTabs: true });
  const shown = await env.newTab("demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [shown], "the show-tabs session's tab is in front");
  return { ...env, shown };
}

test("another client's session opens its tabs in the background and raises no window", async () => {
  const env = await both();
  const first = await env.newTab("work", PLAIN);
  assert.deepEqual(env.browser.activeIn(10), [env.shown], "its first tab doesn't take the front");
  env.browser.userFocusesWindow(20);
  const second = await env.newTab("work", PLAIN);
  assert.deepEqual(env.browser.activeIn(10), [env.shown], "nor does the next");
  assert.equal(env.browser.focusedWindow(), 20, "the user's window stays in front");
  assert.equal(env.browser.tabsMap.get(first).active, false);
  assert.equal(env.browser.tabsMap.get(second).active, false);
  assert.notEqual(env.browser.groupOf(first), env.browser.groupOf(env.shown), "each session has its own group");
});

test("a client that sent showTabs: false is like one that sent nothing", async () => {
  const env = await both({ showTabs: false });
  await env.newTab("work", PLAIN);
  assert.deepEqual(env.browser.activeIn(10), [env.shown]);
});

test("another client's calls don't select the tab they act in", async () => {
  const env = await both();
  const tab = await env.newTab("work", PLAIN);
  await env.browser.userSelects(1);
  await env.call("find", { tabId: tab, query: "x" }, "work", PLAIN);
  await env.call("computer", { action: "left_click", tabId: tab, coordinate: [10, 10] }, "work", PLAIN);
  assert.deepEqual(env.browser.activeIn(10), [1], "the user stays on their tab");
});

for (const together of [false, true]) {
  test(`focus is still handed back when another client's page opens a tab (${together ? "activated before onCreated is done" : "created, then activated"})`, async () => {
    const env = await both();
    const tab = await env.newTab("work", PLAIN);
    await env.browser.userSelects(1);
    const opened = await env.browser.pageOpensTab(tab, { together });
    assert.deepEqual(env.browser.activeIn(10), [1], "back on the tab the user was on");
    assert.equal(env.browser.groupOf(opened), env.browser.groupOf(tab));
  });
}
