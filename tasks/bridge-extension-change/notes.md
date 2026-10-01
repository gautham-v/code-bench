# bridge-extension-change

Feature: demo-only "show tabs" mode in the extension (a show-tabs client's session works in
front instead of in background tabs).

## Commit

- base `6e8c08a` (parent of the fix), fix `43fbae6` "Demo-only FIREFOX_BRIDGE_SHOW_TABS: a
  session's tabs open active in the front window and stay selected".
- The fix also touches `host/host.mjs` and `mcp/server.mjs` (passing the flag along). The task
  is only the extension half: the prompt says the host will send `client.showTabs: true` on the
  connected event and that the owner wires host and MCP himself. No check runs host/ or mcp/.
- Why this one: it is the only extension feature in the history whose whole behavior shows in the
  mocked `browser` (which tab is active, which window is focused) and needs one new name, the
  `showTabs` key, which the prompt gives. The others I read need a spec's worth of protocol in
  the prompt (find ranking: `roleHit`, `rest`, a new module's signature; navigate wait
  "interactive": a new actor op and exact result strings; agent cam masks: a panel command and
  the `maskRects` answer shape), lean on wall-clock bounds (click wait, navigate wait), or are
  bugs (one group for parallel first tabs, captures one at a time, group numbering).
- No spoiler at base: nothing in the tree mentions showTabs / SHOW_TABS; the race-video docs and
  scripts come after the fix.

## What makes it hard

The flag bookkeeping has a template at base (devtools clients/sessions). The reasoning is in
finding every place focus is decided in `background.js`:

- three ways a tab is opened for a session (tabs_create_mcp; the first tab of navigate and of
  tabs_context_mcp createIfEmpty), two code paths (new group, existing group), and the window
  has to be raised when the group's window isn't the focused one;
- re-selecting on a call has to come after the tab is checked against the session's group;
- focus is handed back in three places (onActivated with `justOpened`, onActivated before
  onCreated has finished, and the end of onCreated). A guard in one or two of them works in the
  easy event order only;
- the mode is per client id, two clients can share a name, and it has to end on disconnect.

## Checks and the wrong fixes they catch

I ran ten hand-written variants against the hidden tests (reference re-typed, a different
correct design, eight wrong ones):

| variant | fails |
| --- | --- |
| only opens tabs in front | acted_tab_selected_again, page_opened_tab_stays_in_front |
| no `windows.update(..., { focused: true })` | new_tabs_open_in_front |
| one global switch while any show-tabs client is connected | other_clients_sessions_unchanged |
| keyed by client name | other_clients_sessions_unchanged, ends_when_client_disconnects |
| session stays shown after its client left | ends_when_client_disconnects |
| hand-back skipped only in onCreated | page_opened_tab_stays_in_front (the "activated before onCreated is done" case) |
| no hand-back guard | page_opened_tab_stays_in_front |
| re-select in the call path before the group check | acted_tab_selected_again (a tab outside the group gets selected) |

Most likely wrong fix: skipping the hand-back where onCreated ends (or only via `justOpened`),
which leaves the path the base comment calls "Activated before onCreated ran for it".
`page_opened_tab_stays_in_front` catches it.

The different correct design (flag kept on control.js's client record and asked through
`control.sessionInfo`, tab created in the background then selected) passes all five feature
checks. It fails `existing_extension_tests_pass`, because control.test.js compares
`sessionInfo` and the client list with deepEqual. That is the protected check doing its job,
but worth knowing when reading results. The prompt now ends with "The existing tests should
keep passing as they are", so a run knows not to edit them; the same design with the flag kept
in a set of its own inside control.js and a new accessor passes all six checks.

## Review changes (independent check)

- `ends_when_client_disconnects` passed for a solution that never handles the disconnected
  event: show-tabs client ids kept in a set forever, a session shown when its latest call's
  client id is in it. Every test made a call from the reconnected client first, and the host
  gives a reconnect a new id, so nothing depended on the disconnect. Added two tests (both
  event orders): the client disconnects, nobody reconnects or calls, the session's page opens
  a tab, and focus is handed back. That variant now fails; the reference and the control.js
  design pass.
- Prompt, second bullet: "the next call that acts in one of the session's tabs" left room for
  re-selecting on clicks and typing only, while the tests re-select on find and get_page_text.
  It now says the session's next call on one of its tabs, whatever the call is, reading
  included. A computer-only variant fails `acted_tab_selected_again`.
- Prompt: added the line about the existing tests (above).

## What the tests assume

- `hidden/extension/test/show-tabs-env.js` is the mock, a copy of background.test.js's with
  Firefox's selection rules added: one active tab per window, `tabs.create` active unless
  `active: false`, `tabs.update(id, { active: true })` selects, both fire `tabs.onActivated`
  (the base mock fires nothing), one focused window moved by
  `windows.update(id, { focused: true })`. It loads the scripts listed in manifest.json's
  `background.scripts`, so a solution that adds a script file still loads.
- Selecting is expected through `tabs.create({ active })` or `tabs.update(id, { active: true })`
  and raising through `windows.update`. `tabs.highlight` isn't mocked.
- Only state is asserted (active tab per window, focused window, group membership, the
  existing "not in this session's tab group" error). No new strings, no helper names.
- The user clicking away must work until the next call (the prompt says "the next call ...
  selects that tab again"), so a solution that snaps back on every onActivated fails
  `acted_tab_selected_again`.
- `other_clients_sessions_unchanged` and `ends_when_client_disconnects` each start by
  asserting that the show-tabs session's tabs_create_mcp tab is in front; otherwise they would
  pass at base. A solution that can't open in front fails those two as well.
- Not tested: a page-opened tab while the user is on a tab outside the session (the prompt
  doesn't settle it), host disconnect, paused sessions.

## existing_extension_tests_pass

All ten base test files are protected and named in the command. background.test.js is changed
by the fix (mock and two new tests), but its base content passes at the fix, so it is protected
too. The file takes 70 s in one process, so the command runs it as two processes at once
(about 37 s): one skips the six slowest tests by name, the other runs only those. Five tests
with tight wall-clock upper bounds (`took < 95` on clicks, `< 900` on navigate wait
"interactive") are left out of both, so the check can't flake when several graders run; they
don't touch tab selection.

Five graders in parallel: every check passed in each, 37 s for the first check and 4-5 s for
the others.
