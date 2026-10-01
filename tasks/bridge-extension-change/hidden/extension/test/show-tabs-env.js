"use strict";

// Shared by the show-tabs tests: the extension's background scripts, loaded the way the manifest
// lists them (one global scope) against a mocked `browser`, as background.test.js does.
//
// The mock keeps what Firefox keeps about which tab is in front:
// - one active tab per window. tabs.create makes its tab active unless told `active: false`, and
//   tabs.update(id, { active: true }) selects a tab; both fire tabs.onActivated when the window's
//   active tab changes, as Firefox does.
// - one focused window. windows.update(id, { focused: true }) raises a window, and
//   windows.getLastFocused answers the focused one.
// The user's side (clicking a tab, switching windows, a page opening a tab) is driven by the
// helpers at the bottom.
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { webcrypto } = require("node:crypto");

const EXT = path.join(__dirname, "..");

const event = () => {
  const listeners = [];
  return {
    addListener: (f) => listeners.push(f),
    removeListener: (f) => listeners.includes(f) && listeners.splice(listeners.indexOf(f), 1),
    hasListener: (f) => listeners.includes(f),
    fire: (...a) => Promise.all(listeners.map((f) => f(...a))),
    listeners,
  };
};

const wait = (ms) => new Promise((r) => setTimeout(r, ms));

// Window 10 holds the user's tab 1 and is focused; window 20 holds the user's tab 5.
function mockBrowser() {
  const tabs = new Map([
    [1, { id: 1, windowId: 10, groupId: -1, active: true, url: "https://user.example/", status: "complete", title: "user", width: 800 }],
    [5, { id: 5, windowId: 20, groupId: -1, active: true, url: "https://other.example/", status: "complete", title: "other", width: 800 }],
  ]);
  const windows = new Map([
    [10, { id: 10, incognito: false, type: "normal" }],
    [20, { id: 20, incognito: false, type: "normal" }],
  ]);
  const groups = new Map();
  const store = {};
  let focused = 10;
  let nextTab = 100;
  let nextGroup = 500;
  let nextWindow = 30;
  const native = { sent: [], onMessage: event(), onDisconnect: event() };
  const win = (id) => ({ ...windows.get(id), focused: id === focused });

  // Makes `id` its window's active tab. Answers the tab that was active before, or null when
  // nothing changed.
  const select = (id) => {
    const t = tabs.get(id);
    if (!t || t.active) return null;
    const before = [...tabs.values()].find((o) => o.windowId === t.windowId && o.active);
    for (const o of tabs.values()) if (o.windowId === t.windowId) o.active = false;
    t.active = true;
    return { previousTabId: before?.id };
  };
  // A selection made through the API is announced the way Firefox announces it.
  const selectAndAnnounce = (id) => {
    const changed = select(id);
    if (changed) setTimeout(() => b.tabs.onActivated.fire({ tabId: id, previousTabId: changed.previousTabId, windowId: tabs.get(id)?.windowId }).catch(() => {}), 0);
  };

  const b = {
    native,
    store,
    tabsMap: tabs,
    groups,
    runtime: {
      connectNative: () => ({ onMessage: native.onMessage, onDisconnect: native.onDisconnect, postMessage: (m) => native.sent.push(m) }),
      getManifest: () => ({ version: "0.1.0" }),
      getURL: (p) => `moz-extension://x/${p}`,
      onConnect: event(),
      onMessage: event(),
    },
    storage: { local: { get: async (k) => ({ [k]: store[k] }), set: async (o) => Object.assign(store, o), remove: async (k) => delete store[k] } },
    omnibox: { onInputStarted: event(), onInputChanged: event(), onInputEntered: event(), setDefaultSuggestion: () => {} },
    notifications: { create() {}, clear() {}, onClicked: event() },
    windows: {
      onFocusChanged: event(),
      onRemoved: event(),
      onCreated: event(),
      WINDOW_ID_NONE: -1,
      get: async (id) => {
        if (!windows.has(id)) throw new Error("no window");
        return win(id);
      },
      getAll: async () => [...windows.keys()].map(win),
      getCurrent: async () => win(focused),
      getLastFocused: async () => win(focused),
      update: async (id, o = {}) => {
        if (!windows.has(id)) throw new Error("no window");
        if (o.focused === true) focused = id;
        return win(id);
      },
      create: async () => {
        const id = nextWindow++;
        windows.set(id, { id, incognito: false, type: "normal" });
        return win(id);
      },
    },
    sidebarAction: { toggle: () => {}, open: async () => {}, close: async () => {}, setIcon: async () => {} },
    tabs: {
      TAB_ID_NONE: -1,
      onActivated: event(),
      onCreated: event(),
      onUpdated: event(),
      onRemoved: event(),
      onAttached: event(),
      onDetached: event(),
      onMoved: event(),
      query: async (q = {}) =>
        [...tabs.values()]
          .filter(
            (t) =>
              (q.active == null || t.active === q.active) &&
              (q.windowId == null || t.windowId === q.windowId) &&
              (q.groupId == null || t.groupId === q.groupId) &&
              (q.currentWindow == null || (t.windowId === focused) === q.currentWindow) &&
              (q.lastFocusedWindow == null || (t.windowId === focused) === q.lastFocusedWindow),
          )
          .map((t) => ({ ...t, index: t.id })),
      get: async (id) => {
        if (!tabs.has(id)) throw new Error("no tab");
        return { ...tabs.get(id), index: id };
      },
      create: async ({ url = "about:blank", windowId = focused, active = true, openerTabId } = {}) => {
        const t = { id: nextTab++, windowId, groupId: -1, active: false, url, status: "complete", title: "" };
        if (openerTabId != null) t.openerTabId = openerTabId;
        tabs.set(t.id, t);
        if (active) selectAndAnnounce(t.id);
        return { ...t, index: t.id };
      },
      update: async (id, props = {}) => {
        if (!tabs.has(id)) throw new Error("no tab");
        const { active, highlighted, ...rest } = props;
        Object.assign(tabs.get(id), rest);
        if (active === true) selectAndAnnounce(id);
        return { ...tabs.get(id), index: id };
      },
      group: async ({ tabIds, groupId, createProperties }) => {
        const ids = Array.isArray(tabIds) ? tabIds : [tabIds];
        const gid = groupId ?? nextGroup++;
        if (!groups.has(gid)) groups.set(gid, { id: gid, windowId: createProperties?.windowId ?? tabs.get(ids[0]).windowId, title: "" });
        for (const id of ids) tabs.get(id).groupId = gid;
        return gid;
      },
      remove: async (id) => tabs.delete(id),
      captureTab: async () => "data:image/jpeg;base64,AAAA",
      getZoom: async () => 1,
      goBack: async () => {},
      goForward: async () => {},
      ungroup: async (id) => {
        const gid = tabs.get(id).groupId;
        tabs.get(id).groupId = -1;
        if (![...tabs.values()].some((t) => t.groupId === gid)) groups.delete(gid);
      },
    },
    tabGroups: {
      get: async (id) => {
        if (!groups.has(id)) throw new Error("no group");
        return { ...groups.get(id) };
      },
      update: async (id, props) => Object.assign(groups.get(id), props),
      query: async () => [...groups.values()],
      onMoved: event(),
      onRemoved: event(),
      onUpdated: event(),
      onCreated: event(),
    },
    webRequest: { onBeforeRequest: event(), onBeforeRedirect: event(), onCompleted: event(), onErrorOccurred: event() },
    claudePage: {
      devtoolsWatch: async (tabIds) => tabIds.length,
      onConsole: event(),
      onPick: event(),
      record: async () => {},
      onRecord: event(),
      setActive: async () => {},
      keepRendering: async () => {},
      call: async (tabId, op) => {
        if (op === "textSize") return 10;
        if (op === "viewport") return { width: 1000, height: 800, dpr: 1, scrollX: 0, scrollY: 0, screenX: 0, screenY: 0, title: "t", url: "https://t.example/" };
        if (op === "readyState") return { id: 1, state: "complete", title: "t" };
        return "done";
      },
      broadcast: async () => [],
      setGroupState: async (groupId) => groups.has(groupId),
    },
    browserAction: { onClicked: event(), setIcon: () => {}, setTitle: () => {}, setBadgeText: () => {}, setBadgeBackgroundColor: () => {} },
    commands: { onCommand: event() },

    // ---- what the test reads and drives

    // The tab in front in a window, and the window in front.
    activeIn: (windowId) => [...tabs.values()].filter((t) => t.windowId === windowId && t.active).map((t) => t.id),
    focusedWindow: () => focused,
    groupOf: (tabId) => tabs.get(tabId)?.groupId,

    // The user clicks a tab in the tab strip.
    async userSelects(tabId) {
      const changed = select(tabId);
      if (changed) await b.tabs.onActivated.fire({ tabId, previousTabId: changed.previousTabId, windowId: tabs.get(tabId).windowId });
      await wait(20);
    },
    // The user switches to another Firefox window.
    userFocusesWindow(windowId) {
      focused = windowId;
    },
    // A page in tab `opener` opens a new tab (a target=_blank link, window.open) and Firefox
    // brings it to the front. The extension hears tabs.onCreated and tabs.onActivated for it:
    // one after the other, or with `together` both at once, onActivated first, as happens when
    // Firefox activates the tab before the extension has finished with onCreated. Answers the new
    // tab's id once the extension has had time to react.
    async pageOpensTab(opener, { together = false } = {}) {
      const from = tabs.get(opener);
      const t = { id: nextTab++, windowId: from.windowId, groupId: -1, active: false, openerTabId: opener, url: "https://opened.example/", status: "complete", title: "opened" };
      tabs.set(t.id, t);
      const { previousTabId } = select(t.id);
      const created = () => b.tabs.onCreated.fire({ ...t, active: true, index: t.id });
      const activated = () => b.tabs.onActivated.fire({ tabId: t.id, previousTabId, windowId: t.windowId });
      if (together) await Promise.all([activated(), created()]);
      else {
        await created();
        await activated();
      }
      await wait(60);
      return t.id;
    },
  };
  return b;
}

async function load() {
  const browser = mockBrowser();
  const matchMedia = () => ({ matches: false, addEventListener: () => {}, removeEventListener: () => {} });
  const ctx = vm.createContext({
    browser,
    console,
    setTimeout,
    clearTimeout,
    setInterval,
    clearInterval,
    queueMicrotask,
    structuredClone,
    URL,
    URLSearchParams,
    Date,
    Promise,
    matchMedia,
    crypto: webcrypto,
    atob,
    btoa,
    TextEncoder,
    TextDecoder,
  });
  const scripts = JSON.parse(fs.readFileSync(path.join(EXT, "manifest.json"), "utf8")).background.scripts;
  for (const f of scripts) vm.runInContext(fs.readFileSync(path.join(EXT, f), "utf8"), ctx, { filename: f });
  await wait(20);

  const replies = () => browser.native.sent.filter((m) => m.result);
  let n = 0;
  // A tool call from `client` in `session`, as the native host passes it on. Answers the result's
  // text; a refused or failed call throws unless `mayFail`.
  const call = async (tool, args, session, client, { mayFail = false, ms = 8000 } = {}) => {
    const id = `${client.id}:${++n}`;
    await browser.native.onMessage.fire({ type: "call", id, session, tool, args, client: { id: client.id, name: client.name } });
    for (let i = 0; i < ms / 10 && !replies().some((r) => r.id === id); i++) await wait(10);
    const reply = replies().find((r) => r.id === id);
    assert.ok(reply, `${tool} got no answer`);
    const text = reply.result.content.map((c) => c.text ?? "").join("\n");
    if (!mayFail) assert.ok(!reply.result.isError, `${tool} failed: ${text}`);
    // Selections the call made are announced a moment later.
    await wait(30);
    return { text, isError: !!reply.result.isError };
  };
  // The host's client events. `flags` are extra fields of the connected event's client.
  const connect = async (client, flags = {}) => {
    await browser.native.onMessage.fire({ type: "client", event: "connected", client: { id: client.id, name: client.name, version: "2", pid: 1000 + client.id, cwd: "/p", ...flags } });
    await wait(10);
  };
  const disconnect = async (client) => {
    await browser.native.onMessage.fire({ type: "client", event: "disconnected", client: { id: client.id } });
    await wait(30);
  };
  // tabs_create_mcp; answers the new tab's id.
  const newTab = async (session, client) => {
    const { text } = await call("tabs_create_mcp", {}, session, client);
    const id = Number(text.match(/Created tab (\d+)/)?.[1]);
    assert.ok(browser.tabsMap.has(id), text);
    return id;
  };
  return { browser, call, connect, disconnect, newTab };
}

// Two clients with the same name, as two Claude Code windows have: only SHOW's MCP server asked
// for its tabs to be shown.
const SHOW = { id: 2, name: "claude-code" };
const PLAIN = { id: 1, name: "claude-code" };

module.exports = { load, wait, SHOW, PLAIN };
