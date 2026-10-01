# catcher-deep-bug

## Commits

- base `84f3c9f` (ui: keep truncate_left above the test module), the commit right after eight
  feature branches were merged into `integrate` on 2026-09-01.
- fix `49852c4` (Merge branch 'feat/fold-preview'), nine commits later. 428 added / 30 removed
  across 7 files. The ones the task is about:
  - `4fb4c19` a disk reload settles the folds and puts the cursor on a line that is on screen
  - `1e6bc59` a note's folds follow its file when a save, a rename or a move changes the path
  - `ed62918` a search hit inside a fold opens it, and a failed open leaves the cursor alone
  - `82a14cd` the hash in `[[#heading]]` is not a tag: styling, cursor and index all agree
  - `b30b4fe` the contents tab searches an open note as it stands in the buffer
  - `48fdd6e` CATCHER_DIR is not written into settings.md as the notes folder
  - `c170902` the reading view folds too (added in attempt 3 to widen the scope)
  - `c94d277` (daily note re-walks the index) is in the range but not asked for or tested: it is
    a one-line fix that would only add an easy check.

Why this cluster: the features were written on separate branches and each works alone. Every bug
is where two of them meet (folding x outside edits, folding x rename, folding x search, search x
autosave, tags x wikilinks, settings catch-up x environment). The owner fixed them one after
another in 25 minutes, and three of the fixes carry a second fix for something the first would
have got wrong (cursor inside a moved fold, failed open, styling and cursor agreeing with the
index).

Passed over in the territory: the v0.x release squashes (bug fixes can't be separated from the
features), `009a481` (superseded key defaults: the old defaults would have to be spelled out in
the prompt), the tilde-fence unification in `1a9ac33` (needs the refactor commits as the fix, and
is a grep-and-replace).

## What makes it hard

- Six symptoms, nine places to change, in app.rs, fold.rs, md.rs, config.rs, ui.rs/render.rs.
- The prompt gives symptoms only. Three of them have a cause that is not where the symptom shows:
  - "a different line" after a search hit: the list is read from disk while the note opens from
    the buffer, which can be an autosave ahead.
  - `[[#Setup]]` listed under a tag: the listing is the reported symptom, the cause is `tag_at`,
    which the styling, the cursor and the reading view share. Fixing only the index scan
    (`tags_in`) leaves the link drawn and followed as a tag, against the comment on `tags_in`
    ("what is drawn as a tag and what the index counts as one are the same set").
  - /tmp/demo in settings.md: written by the catch-up rewrite in `Config::load`, which only runs
    when the file is missing a newer setting.
- Fixes that create the next bug: once folds are re-settled after a reload, the cursor (kept by
  line number) can sit inside a fold; the base code's rule is that it never does (`leave_folds`,
  `fold_all`).
- Paths nobody lists: the file is also renamed by a save when the title changes; Enter on a hit
  whose file has gone moves the cursor in the note that stayed on screen.

## Calibration (Sonnet 5.5, low effort, two runs each)

| attempt | scores | failed checks | change made after |
| --- | --- | --- | --- |
| v1 | 0.8, 0.7 | cursor (both), gone (both), unsaved (one) | One run passed `unsaved` by following the hit's text to the nearest matching line, which still opens a stale hit on the wrong line: added a test for a line retyped since the last save, a symlinked notes folder, nested folds. Tag bullet reworded to the symptom the owner saw (the listing) instead of "is treated as the tag". |
| v2 | 0.8, 0.7 | gone (both), tag (both), cursor (one) | Same mean, so widened the range by one commit: the reading view shows the editor's folds, click on a heading toggles. Rename/move and title-rename checks merged to stay at ten. |
| v3 | 0.7, 0.8 | gone (both), tag (both), cursor (one) | Both runs hid folded lines in the reading view with nothing on the heading to say so. Added a test that a folded heading's row differs from the open one (any marker, count or style), and the prompt now says "show them the way the editor does". |
| v4 | 0.7, 0.6 | gone (both), tag (both), unsaved (both), cursor (one) | final |

All eight runs failed; none took more than 110 seconds. v4 mean 0.65. Both v4 runs added a marker
in the reading view, so the v3 change cost them nothing; the lower score comes from both blaming
the wrong-line symptom on the index being re-walked under the open tab and leaving the
buffer-versus-disk cause alone (three of the six earlier runs found it). Expect Sonnet low around
0.7 and never passing.

Every miss read against its patch:
- `cursor_on_screen_after_outside_edit`: settle + refresh_visible added to the reload, nothing for
  the cursor. It ends on a hidden line: no cursor is drawn and typing goes into text off screen.
- `search_hit_gone_keeps_cursor`: no run checked that the open succeeded; the cursor of the note
  still on screen jumps to the hit's line number.
- `heading_link_is_not_a_tag`: runs that fixed `tags_in` only. The listing test inside the check
  passes; the link is still drawn in the tag colour and alt+enter on it opens the tag list. Two
  runs claimed the styling used the same function. It does not.
- `search_hit_matches_unsaved_buffer`: the reported symptom, reproduced with two unsaved lines
  above the hit.

## Likely wrong fixes and the check that catches each

| wrong fix | check |
| --- | --- |
| reload only rebuilds `visible`, or drops every fold | folds_follow_outside_edit |
| reload settles folds, cursor left where it was | cursor_on_screen_after_outside_edit |
| folds carried over in rename and move but not in the save that follows the title | folds_follow_renamed_or_moved_file |
| reading view cuts the source instead of the rows (line numbers shift), hides paragraphs only, or gives a folded heading no sign | reading_view_has_the_folds |
| search hit: only the nearest fold opened | search_hit_in_fold_is_shown |
| search hit: line found by text match, raw path compare, or cause not found | search_hit_matches_unsaved_buffer |
| cursor set after an open that failed | search_hit_gone_keeps_cursor |
| `tags_in` skips wikilinks, `tag_at` untouched | heading_link_is_not_a_tag |
| only the first-run write of settings.md fixed | env_dir_not_written_to_settings |

Tried by hand on the base: reload with settle but no cursor handling fails only the cursor check;
the same plus `reveal_cursor` passes it; `save_now()` before the search passes the unsaved check;
relocate in rename and move only fails the title-rename tests.

## What the tests assume

- One hidden module, `src/heldout.rs`, declared in `src/main.rs` by `heldout-check.sh`. It uses
  only what is public at base: `App::launch`, `on_key`, `on_mouse`, `tick`, `open_path`,
  `folded_here`, `visible`, `editor`, `edit_rows`, `preview_checkboxes`, `overlay`, `contents`,
  `selected`, `overlay_items`, `tag_filter`, `open_index`, `ui::draw` into a `TestBackend`,
  `Config::load`, `md::tags_in`, `md::link_at`, `md::style_line`, `notes::slug`.
- Each test sets HOME to a directory of its own under `$CARGO_TARGET_DIR/heldout-tmp` and clears
  CATCHER_DIR, under one lock, and the checks run with `--test-threads=1`.
- No wall-clock limits. An outside edit and an autosave rename are waited for by calling `tick()`
  every 150 ms until the buffer or the path changes (up to 9 s); outside writes always change the
  file's length, so the mtime's resolution does not matter.
- Either design passes where two are reasonable: the cursor may go to the heading or the fold may
  open for it; search may read the buffer or save before reading; a stale hit may be absent or
  land on a line that has the word; `[[#Setup]]` may be plain text, a link, or followable, as long
  as it is not a tag; settings.md may be rewritten or left alone, as long as the environment's
  folder is not in it; the reading view's marker can be anything visible.
- Not tested, because the reference picks arbitrarily or leaves it broken: the first-run write of
  settings.md (the reference still leaks there), fold keys in the reading view, blank rows around
  a folded section, the marker's glyph and count, a heading click under `preview_click: edit`.
- `existing_tests_pass` runs the 419 test names that exist at base with `--exact` and wants all
  419 to pass. They are inline `#[cfg(test)]` modules in the files the fix edits, so `protected`
  is empty: restoring those files would undo the run's own change. A run that deletes or renames
  one of them fails the count. The existing tests write to fixed names under the temp dir, so the
  script points TMPDIR at the run's own directory.

## Limits

The bugs are shallow one at a time: Sonnet low fixes most of them in a minute or two. The
difficulty is the edges and the two symptoms with a cause away from where they show. Stronger
configs will probably miss fewer; the failed open and the cursor after a reload are the checks I
would expect to hold out longest.

## Independent check (2026-09-30)

- Blind solve from the prompt and the base code only: all ten checks passed first time, about
  nine minutes. The solution differs from the reference where two designs are reasonable (folds
  carried across a reload by lining the old and new buffers up rather than by heading text; the
  search saves before reading the disk rather than reading the buffer; the reading view filters
  rendered rows in a new `render::fold_page`), so the tests do not depend on the reference's shape.
- Every v4 miss re-read against its patch and judged a real defect, not an arbitrary demand. The
  one to keep an eye on is `search_hit_gone_keeps_cursor`: the scenario (the hit's file removed
  before Enter) is not one of the six listed symptoms, only an unchecked failure in the function
  the search fix has to edit. The expected behaviour is not a choice (the alternative moves the
  cursor in an unrelated note), so it stays, but a config that fails only this check failed a
  code-reading edge, not a stated requirement.
- One repair, to `heldout-check.sh`: the `existing` branch now touches `src/main.rs` first. An
  export keeps its commit's file times, which are older than the warmed build, so at `fix` cargo
  called the workspace fresh and `verify` ran the cached base binary (419 passed, 0 filtered out,
  0.3 s). With the touch it builds the fix (419 passed, 7 filtered out). A real run was never
  affected: any edit makes the crate newer than the cache.
- Known limit, not repaired: the existing tests are inline, so `protected` cannot restore them and
  a run could pass `existing_tests_pass` by editing an assertion. No calibration run did.
