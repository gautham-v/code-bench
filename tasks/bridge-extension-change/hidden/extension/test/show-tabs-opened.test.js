"use strict";

// Show tabs: a tab that a page in the session's tab opens (target=_blank, window.open) is left in
// front, instead of focus going back to the tab that was selected before.
const test = require("node:test");
const assert = require("node:assert/strict");
const { load, SHOW, PLAIN } = require("./show-tabs-env.js");

for (const together of [false, true]) {
  test(`a tab the session's page opens stays in front (${together ? "activated before onCreated is done" : "created, then activated"})`, async () => {
    const env = await load();
    await env.connect(PLAIN);
    await env.connect(SHOW, { showTabs: true });
    const tab = await env.newTab("demo", SHOW);
    // The session's tab is the one on screen (it opened in front; if not, the user selects it).
    await env.browser.userSelects(tab);
    assert.deepEqual(env.browser.activeIn(10), [tab]);
    const opened = await env.browser.pageOpensTab(tab, { together });
    assert.deepEqual(env.browser.activeIn(10), [opened], "the new tab is still the selected one");
    assert.equal(env.browser.groupOf(opened), env.browser.groupOf(tab), "and it joined the session's group");
  });
}

test("it stays in front after the user had been on another tab and come back to the session's", async () => {
  const env = await load();
  await env.connect(SHOW, { showTabs: true });
  const tab = await env.newTab("demo", SHOW);
  await env.browser.userSelects(1);
  await env.browser.userSelects(tab);
  assert.deepEqual(env.browser.activeIn(10), [tab]);
  const opened = await env.browser.pageOpensTab(tab);
  assert.deepEqual(env.browser.activeIn(10), [opened]);
  // The session goes on in the new tab, which is one of its own now.
  await env.call("find", { tabId: opened, query: "x" }, "demo", SHOW);
  assert.deepEqual(env.browser.activeIn(10), [opened]);
});
