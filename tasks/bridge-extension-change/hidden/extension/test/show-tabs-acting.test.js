"use strict";

// Show tabs: after the user clicks away, the next call that acts in one of the session's tabs
// selects that tab again.
const test = require("node:test");
const assert = require("node:assert/strict");
const { load, SHOW, PLAIN } = require("./show-tabs-env.js");

test("a call in the session's tab brings it back after the user selected another tab", async () => {
  const env = await load();
  await env.connect(PLAIN);
  await env.connect(SHOW, { showTabs: true });
  const tab = await env.newTab("demo", SHOW);
  await env.browser.userSelects(1);
  assert.deepEqual(env.browser.activeIn(10), [1], "the user can click away");
  await env.call("find", { tabId: tab, query: "search box" }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [tab], "find");
  await env.browser.userSelects(1);
  await env.call("computer", { action: "left_click", tabId: tab, coordinate: [10, 10] }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [tab], "a click");
  await env.browser.userSelects(1);
  await env.call("get_page_text", { tabId: tab }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [tab], "get_page_text");
});

test("with several tabs, the one the call acts in is the one selected", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const a = await env.newTab("demo", SHOW);
  const b = await env.newTab("demo", SHOW);
  await env.browser.userSelects(b);
  await env.call("find", { tabId: a, query: "x" }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [a]);
  await env.call("find", { tabId: b, query: "x" }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [b]);
});

test("a call naming a tab outside the session's group is refused as before and selects nothing", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  await env.newTab("demo", SHOW);
  // Another of the user's tabs, in the background of the same window.
  env.browser.tabsMap.set(7, { id: 7, windowId: 10, groupId: -1, active: false, url: "https://mail.example/", status: "complete", title: "mail" });
  await env.browser.userSelects(1);
  const r = await env.call("find", { tabId: 7, query: "inbox" }, "demo", SHOW, { mayFail: true });
  assert.equal(r.isError, true);
  assert.match(r.text, /not in this session's tab group/);
  assert.deepEqual(env.browser.activeIn(10), [1], "the user's tab 7 isn't brought to the front");
});
