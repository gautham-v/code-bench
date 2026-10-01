# squawk-hard

## Commits

- base `e0ae37c` (Scaffold squawk workspace)
- fix `e9a4315` (Add Claude Code mode: session detection, @mentions and repo vocabulary):
  2997 lines added across 11 files, one commit, all in `crates/squawk-core/src/context/`.

At base `squawk_core::context` is the agreed public API with stub bodies: `detect` returns `None`,
`RepoVocab::build` an empty vocab, `VocabCache::get` rebuilds every time, `apply` returns its
input. Each stub's doc comment says what it has to do. The fix fills all four in.

Why this one. squawk's history is 35 commits and almost all of them are features that add their
own new API, which a held-out test cannot call without the prompt dictating it. The few real bug
fixes are small or untestable here:

- `8e8a73b` (keep a leading "so"): one function, 20 lines.
- `b59949f` (🌐 key's own keyDown): needs the fact that the key sends keycode 179, which no one
  can work out from the code.
- `a9913d2` (mic dropouts, stalled downloads): hardware, wall-clock timeouts, a 60 s body window.
- `ef53a61`, `7c2d66e` (segment lengths, dither): tuned against the real model.
- `9770609` (remove "Stop when the call ends"): mechanical removal.
- `abfafd6` (record automatically), `79ceda9` (notetaker core), `b3db84c` (echo): `tick` changes
  its return type, new enums, new functions; the rules are the owner's product decisions (5
  minutes early, 8 s panel, 0.75 word share), so a fair prompt would have to spell them out.

Claude Code mode is the one large piece whose interface already exists at base, is called by the
rest of the workspace, and whose behavior follows from what the feature is for. It builds with
`-p squawk-core` only (about 6 s cold, no gpui, no ONNX, no llama.cpp).

## What makes it hard

Scope and edge cases, not diagnosis. Four pieces have to work and agree:

1. `detect`: walk the macOS process table (libproc FFI) down from the terminal's pid to a
   `claude`/`codex` process, and read its cwd.
2. `RepoVocab::build`: the repo's files relative to the session's cwd (git, or a walk outside
   git) and its jargon (package names, terms from README.md / CLAUDE.md).
3. `VocabCache`: reuse until the repo's index changes.
4. `apply`: spoken file names to `@path`, repo terms to the repo's casing, never touching
   protected chunks, never guessing.

Sonnet at low effort writes a compact version of all four in 3 to 5 minutes (500 to 900 lines)
and gets the plain cases right every time. What it misses are the cases where a reference has to
be taken as a whole or not at all, which is what "a wrong @mention is worse than none" asks for.

## Calibration (Sonnet 5.5, low effort, 2 runs each)

| attempt | checks | scores | failed |
| --- | --- | --- | --- |
| v1 | 8 | 1.0, 0.75 | run 1: vocab (my test was wrong), cache (worktree) |
| v2 | 10 | 0.5, 0.5 | dotted, folder, protected, repo terms, detect (both runs) |
| v3 | 10 | 0.7, 0.6 | dotted, folder, detect (both); unsure (one) |

**v1.** One run passed everything in 4.5 minutes. The other failed two checks:
- `vocab_lists_the_repos_files`: it drops file names with a space on purpose (they cannot be one
  @mention), and my test asserted `docs/My Notes.md` was listed. Not a defect: the test was wrong.
  Removed; the non-ASCII names stay.
- `vocab_cache_follows_the_repo`: it looks for `.git/index` only, so in a linked worktree (`.git`
  is a file) the cache never sees the index move. Genuine.

Reading both patches showed defects no test reached. Both turned "copy config dot json dot
example" into `@config.json dot example` (a mention of the wrong file), and "store slash audio dot
rs" (audio.rs is not in store/) into `store slash @src/audio.rs`. One turned "run go test" into
`@cmd/run.go test`. One could not resolve `.gitignore`. Changed after v1: added those cases, split
the mention tests into four checks (plain, dotted names, spoken folder, unsure) so the plain cases
keep their credit, and added a detect case for an agent that runs another agent under itself.
Considered and left out as not derivable: spoken "underscore"/"dash", "the makefile", "readme"
without an extension, package.json names, "tide well" joining into `Tidewell`, preferring the
tty's atime over its mtime, a versioned native install whose process name is `2.1.3`.

**v2.** Both failed five checks. Three were the new cases. The other two were one cause counted
twice: neither run collected `GPUI` from "Widget talks to Tidewell over GPUI." (one skips all-caps
words on purpose, one drops any word followed by a period), and every jargon test and one
protected test used `gpui`. Missing an acronym the repo writes is a genuine miss ("the repo's own
terms should come out the way the repo writes them"), but it should cost one check. Changed after
v2: the jargon and protected tests use `Tidewell`/`Brightwave`; the acronym has one test of its
own in `repo_terms_spelled_like_the_repo`. Regraded, the v2 patches score 0.6 each.

**v3.** Final. 0.7 and 0.6, mean 0.65, both fail. Every miss is a wrong result, not a taste call:
- `dotted_names_taken_whole` (both): "delete audio dot rs dot orig" becomes `delete @src/audio.rs
  dot orig`; one also gives `copy @config.json dot example to @config.json`.
- `spoken_folder_has_to_match` (both): `open store slash @src/audio.rs`.
- `unsure_references_stay_as_spoken` (one): `then @cmd/run.go test again`.
- `finds_the_agent_session` (both): with claude running another agent in a second directory,
  `detect` returns the child and its directory.

Across the six runs the existing tests and the plain mention check never failed, and protected
text and the file listing failed only through my two test mistakes above.

## Likely wrong fixes and the check that catches each

| wrong fix | check |
| --- | --- |
| Match `<words> dot <ext>` and stop: the first name that resolves wins, even when "dot example" or "dot orig" follows | dotted_names_taken_whole |
| Split file names on the last dot only, so `config.json.example`, `vite.config.ts` and `.gitignore` never match | dotted_names_taken_whole |
| A spoken folder that matches nothing is dropped and the bare name is tried again | spoken_folder_has_to_match |
| Only one "slash" level | spoken_folder_has_to_match |
| Pick the first of several same-named files | unsure_references_stay_as_spoken |
| Any extension in the repo may be said without "dot" ("run go test") | unsure_references_stay_as_spoken |
| A bare stem ("audio", "main", "cargo", "claude") becomes a mention | unsure_references_stay_as_spoken |
| One-word names only, or snake_case only (ContentView.tsx, build-release.sh) | spoken_file_names_become_mentions |
| Tokenising inside a URL, an existing @mention, a path outside the repo, inline code | protected_text_left_alone |
| Every capitalised word in the README is a term, so "open", "run", "the" get capitals | repo_terms_spelled_like_the_repo |
| Acronyms left out, or a term lost when it ends a sentence | repo_terms_spelled_like_the_repo |
| `git ls-files` without `-z` (quoted non-ASCII names), or paths relative to the git root | vocab_lists_the_repos_files |
| Walking `target/` in a git repo | vocab_lists_the_repos_files |
| TTL-only cache, or `.git/index` only (subdirectory cwd, linked worktree) | vocab_cache_follows_the_repo |
| Children of the terminal only, not descendants; any process, not just claude/codex | finds_the_agent_session |
| Every claude/codex under the terminal is a candidate, the agent's own child included | finds_the_agent_session |
| Weakening or deleting an existing test | existing_tests_pass |

## Checks

| check | file | at base |
| --- | --- | --- |
| existing_tests_pass | lib tests of cleanup, config, dictionary, hotkey, ipc, paths, pipeline, status, store, text (102 at base; fewer fails) | pass |
| spoken_file_names_become_mentions | tests/heldout_mentions.rs | fail |
| dotted_names_taken_whole | tests/heldout_dotted.rs | fail |
| spoken_folder_has_to_match | tests/heldout_folders.rs | fail |
| unsure_references_stay_as_spoken | tests/heldout_unsure.rs | fail |
| protected_text_left_alone | tests/heldout_protected.rs | fail |
| repo_terms_spelled_like_the_repo | tests/heldout_jargon.rs | fail |
| vocab_lists_the_repos_files | tests/heldout_vocab.rs | fail |
| vocab_cache_follows_the_repo | tests/heldout_cache.rs | fail |
| finds_the_agent_session | tests/heldout_detect.rs | fail |

Every "stays as spoken" assertion sits beside one that must convert, so the stub fails each file.

## What the tests assume

- Public API only, as it is at base: `context::{apply, detect, RepoVocab::build, VocabCache,
  Context, Session, FrontApp, Agent}`, `pipeline::finish`, `Dictionary::parse`,
  `CleanupOptions::default`. `RepoVocab` is never built by struct literal (the fix adds a private
  field); every vocab comes from `RepoVocab::build` on a real temp directory, mostly a git repo
  the test creates. Reads `vocab.root`, `.files`, `.words`.
- Left open where the base comments and the fix disagree or the choice is taste:
  - "the claude md": `@CLAUDE.md` with or without "the" kept.
  - A sentence-ending period after a mention may stay or go.
  - File stems as jargon (the base comment lists them, the fix leaves them out): no test input
    has a lowercase word that is a capitalised stem or heading except three, compared ignoring
    case outside the mentions.
  - Untracked and ignored files, headings as terms, files with spaces, same name at different
    depths (the fix picks the shallowest), a written `audio.rs`, the tty field and which of
    several tabs wins: not tested.
- Mentions: "dot" and "slash" are spoken; words join whatever the file name's convention is.
- Jargon: `Tidewell` and `Brightwave` appear mid-sentence, `GPUI` ends a sentence, `widget-core`
  is the Cargo package name. "The", "It", "Open", "Run" only start sentences in the README, and
  "open" and "run" never appear in lowercase there.
- Cache: the index is dated 10 s back before the first `get`, then `git add` moves it. No sleeps,
  no timestamps from the future. Outside git only "a second get right away reuses it".
- detect: real process trees. A copy of the test binary named `claude`, `codex` or `notes` is the
  stand-in (a copy of `/bin/sleep` is killed on Apple silicon, and a symlink keeps the name
  `sleep`); it runs the ignored test `agent_stand_in`, writes its pid and blocks on stdin, so it
  exits when the test ends or dies. Each tree hangs under its own `/bin/sh`, which is the
  "terminal" pid, so parallel tests, parallel graders and the grader's own `claude` ancestors do
  not matter. Waits are for conditions (pid file there, `detect` returns `Some`), with a 60 s /
  15 s ceiling that only a failing solution reaches (the base stub takes 15 s to fail). cwd is
  compared canonicalised. A solution that shells out to `ps` would pass too.
- `protected`: hotkey.rs, ipc.rs, paths.rs, status.rs, store/*.rs. cleanup.rs, text.rs,
  pipeline.rs, config.rs and dictionary.rs are run but not protected: a solution may reasonably
  touch them, and putting them back could stop it compiling. dictionary.rs was protected until
  the independent check: its private `split_jargon` / `is_jargon` are what a jargon pass would
  reuse, and a run that made them `pub(crate)` would have failed every check once the file was
  put back. No calibration run touched it. The existing check skips `context::`
  (the filter `text::` would otherwise match it, and the fix's own timing test lives there).
- Checks set `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, so a global ignore file or
  signing setup cannot change the temp repos. Needs `git` on PATH. macOS only (libproc).
- Three verifies at once pass; no stand-in processes are left behind.

## A harness trap

Every check starts with `touch crates/squawk-core/src/lib.rs`. `prepare` unpacks `git archive`,
which stamps every file with its commit's time, and the warm cache's build is newer than that, so
at `fix` cargo sees nothing to rebuild and runs the base library: verify then reports the fix as
failing. A run's own edits are newer than the cache, so only verify is affected, but any Rust
task with a warm cache has this.

## Independent check (Opus 5.5, blind solve)

Solved from the prompt and the base code only, about 10 minutes, 1,150 lines: 8 of 10 checks.
Missed `dotted_names_taken_whole` (`delete @src/audio.rs dot orig`) and
`spoken_folder_has_to_match` (`open store slash @src/audio.rs`), the same two wrong mentions the
Sonnet runs wrote. Both are defects in the solution, not the tests: the prompt says a wrong
@mention is worse than none. A 20-line change (a reference is dropped when a spoken folder before
it did not match, or "dot <word>" follows it) passed all 10, so the tests do not depend on how
the fix is built. `finds_the_agent_session` passed first time: the walk stops at an agent.

One repair: dictionary.rs taken off `protected` (above). No test changed, so the v3 calibration
stands.
