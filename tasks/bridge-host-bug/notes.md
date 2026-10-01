# bridge-host-bug

## Commit

- base `6e8ecf3` (Start one group when several calls open a session's first tab at once)
- fix `32b38b3` (Chat host: pass sub-agents' calls on under their Task step; add fan-out guidance to the system prompt)

`git log -- host mcp` has no commit that is only a bug fix; everything there is a feature or a
wording change. This one is the closest to a bug as the owner would report it: at base the chat host
throws away every message tagged with `parent_tool_use_id`, so a turn that fans out to sub-agents
shows two steps and then nothing, and each Task/Agent step ends blank. The prompt is written as that
report. Dropping the messages was a deliberate choice at base (there is a comment saying so), so
strictly this is a behavior change asked for as a bug.

Others looked at and passed over:
- `08921bf` (fill in a missing tabId): one file, and a fair prompt would have to spell out a
  dozen arbitrary rules. Hard through exactness.
- `aa7b590` (tool_start carries tabId), `ff664d0` (phone sessions in history): a few lines each.
- `eddd250` (memory and Don't ask settings): a large feature.

The fix commit also adds Task/Agent to `--allowedTools` and a fan-out paragraph to the system
prompt. Neither is asked for in the prompt or tested.

## What makes it hard

About 30 lines across two files, but they have to be the right ones:
- `host/chat.mjs` routes Claude Code's stream. Sub-agent `assistant` messages need their tool_use
  blocks passed on with `parent`, sub-agent `user` messages need to reach the tool_end path, and
  sub-agent text and `stream_event`s must still be dropped (a sub-agent `message_start` let through
  would also reset the top-level stream state).
- `host/chat-format.mjs` `summarizeToolResult` is shared by the live stream and by
  `host/chat-history.mjs`. The reply summary belongs there, or history shows a blank step.
- The reply is the result's text minus the block Claude Code appends, whole (not the first line,
  which is what every other summary takes), at most 300 characters, and only for Task/Agent: other
  tools' output still must not leave the host.

## Most likely wrong fixes

- Deleting the filter line and adding `parent` to the existing tool_start: sub-agent text shows up
  as the agent's. `subagent_chatter_stays_out` catches it (tried: steps and ends pass, quiet fails).
- Building the reply summary in `chat.mjs` where tool_end is emitted: live works, history is blank.
  `reply_shown_in_history` catches it (tried with a differently written, otherwise complete
  solution: every other check passes).
- Joining all text blocks, or taking the first line: `agent_step_shows_reply` (trailer marks appear,
  or the two-line Task reply is cut short).
- Passing on only `assistant` messages: steps never end. `subagent_steps_end`.
- A new sub-agent loop that forgets `HIDDEN_TOOLS`: ToolSearch shows as a step.
  `subagent_chatter_stays_out`.

## Checks

| check | file | expect at base |
| --- | --- | --- |
| existing_tests_pass | format.test.mjs, host.test.mjs, chat-existing.test.mjs | pass |
| subagent_steps_start_under_parent | subagent-steps.test.mjs | fail |
| subagent_steps_end | subagent-ends.test.mjs | fail |
| subagent_chatter_stays_out | subagent-quiet.test.mjs | fail |
| agent_step_shows_reply | subagent-reply.test.mjs | fail |
| reply_shown_in_history | subagent-history.test.mjs | fail |

## What the tests assume

- `host/test/chat.test.mjs` is not protected and not run: the fix changes one assertion in it (the
  exact `--allowedTools` list). `chat-existing.test.mjs` is the base file with that assertion
  relaxed to "the three base tools are there and Bash/Read/Write/Edit/WebFetch/WebSearch are not",
  so a solution passes whether or not it pre-approves Task/Agent. It runs against the workspace's
  `fake-claude.mjs`, which is protected (put back to base before grading).
- The sub-agent tests use their own fake, `subagent-fake-claude.mjs` (base fake plus `[fanout]` and
  `[replies]`), so nothing a run does to `fake-claude.mjs` matters to them. The helper chmods it,
  in case the copy loses the executable bit.
- Wire format in the fake: sub-agent messages are `assistant` / `user` / `stream_event` lines with
  `parent_tool_use_id` set to the Task/Agent call's id; the Task/Agent call's own tool_result is top
  level, an array of text blocks whose last one starts with `agentId:` and holds `<usage>…</usage>`
  (or a plain string with no trailer). The prompt states the trailer's shape.
- Top-level steps may carry `parent: null` or no `parent`. A step that names no tab may carry
  `tabId: null` or none.
- Reply summaries are compared with whitespace collapsed, so keeping line breaks is fine. Length is
  checked as `<= 300` and "starts with the reply's start, at least 200 long"; an ellipsis is fine.
- A failed sub-agent: `ok: false` and a summary that starts with the error's first line (base
  behavior; a longer error summary also passes).
- History is tested only for the Task/Agent step's reply. Sub-agent steps in a reopened chat are
  neither required nor forbidden (the fix doesn't add them, and the test files hold no sidechain
  tool calls).
- Not tested: what happens to a sub-agent's API-error message (the fix ignores it), `--allowedTools`,
  the system prompt.
- Every test sets HOME to its own temp dir; commands run with CLAUDE_CONFIG_DIR and CODEX_HOME
  unset so a grader's own config dirs are never used. No ports. Five verifies at once passed.
  The slowest check is the existing tests, about 10 s.
