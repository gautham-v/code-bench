"use strict";

// Show tabs: the tabs a show-tabs client's session opens are in front, selected in their window
// with the window raised.
const test = require("node:test");
const assert = require("node:assert/strict");
const { load, SHOW, PLAIN } = require("./show-tabs-env.js");

test("tabs_create_mcp opens each of the session's tabs selected, the first and the ones after it", async () => {
  const env = await load();
  await env.connect(PLAIN);
  await env.connect(SHOW, { showTabs: true });
  const first = await env.newTab("demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [first], "the session's first tab is the selected one");
  const second = await env.newTab("demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [second], "so is the next");
  assert.ok(env.browser.groupOf(first) >= 0, "it is still in the session's group");
  assert.equal(env.browser.groupOf(second), env.browser.groupOf(first));
  assert.deepEqual(env.browser.activeIn(20), [5], "the other window is left alone");
});

test("the first tab that navigate opens on its own is selected", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const { text } = await env.call("navigate", { url: "https://a.example/" }, "demo-nav", SHOW);
  const tab = Number(text.match(/^Tab (\d+)/)?.[1]);
  assert.ok(env.browser.tabsMap.has(tab), text);
  assert.deepEqual(env.browser.activeIn(10), [tab]);
});

test("the first tab that tabs_context_mcp createIfEmpty opens is selected", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const { text } = await env.call("tabs_context_mcp", { createIfEmpty: true }, "demo-ctx", SHOW);
  const tab = Number(text.match(/Created tab (\d+)/)?.[1]);
  assert.ok(env.browser.tabsMap.has(tab), text);
  assert.deepEqual(env.browser.activeIn(10), [tab]);
});

test("the window holding the new tab is raised when the user is in another window", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const first = await env.newTab("demo", SHOW);
  assert.equal(env.browser.tabsMap.get(first).windowId, 10);
  // The user goes to their other window; the session's group stays in window 10.
  env.browser.userFocusesWindow(20);
  const second = await env.newTab("demo", SHOW);
  assert.equal(env.browser.tabsMap.get(second).windowId, 10, "the tab joins its group's window");
  assert.deepEqual(env.browser.activeIn(10), [second]);
  assert.equal(env.browser.focusedWindow(), 10, "window 10 is brought to the front");
  assert.deepEqual(env.browser.activeIn(20), [5]);
});
