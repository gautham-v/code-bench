# catcher-big-feature

## Commits

- base `20e459f` (One writer at a time on the clipboard), the parent of the first list commit
- fix `d3356b0` (An ordered list stays in order as it is edited)

Four feature commits squashed (plus three version bumps in between), 2026-09-07 to 2026-09-08:

| commit | what | asked for / tested |
| --- | --- | --- |
| `af10927` A pasted list joins the count of the one it landed in | new `src/lists.rs` (`renumber`), `Editor::insert_paste`, the app's paste calls it | yes |
| `33f0017` Tab nests a list item under the one above it | `lists::shift`, Tab / BackTab in `Editor::on_key`, selection, children carried, undo, cursor | yes |
| `adbf462` A nested list is drawn where the page puts it | `lists::nest`, md.rs / render.rs drawing, `list_guides` setting and palette command | no (see below) |
| `d3356b0` An ordered list stays in order as it is edited | recount after Enter, a delete, a moved line | yes |

Reference diff: 1292 insertions, 37 deletions across 12 files. The part the task asks for is
`src/lists.rs` (new, 712 lines with its tests), `src/editor.rs` (+292), `src/app.rs`, `src/main.rs`:
about 1020 lines.

`adbf462` rides along in the fix but is neither in the prompt nor tested: where a nested item is
drawn (parent's text column + 2, a rule per ancestor) is the owner's design choice, and a test of it
would assert an arbitrary geometry. The base workspace has no mention of any of this (no `lists.rs`,
nothing in the README).

Others looked at and passed over: the trash (`9e24474`, tied to the filesystem and a picker), table
cell wrapping (`74a0857` + `b4079d4`, all rendering geometry), the reading-view vim keys (`f68bd9d`,
one arbitrary key map), `e84a1a6` (live-preview reveal, styling assertions), the pipe-in-table bug
(`5b991d9`, a bug rather than a feature; a good candidate for a separate task).

## Prompt

Symptom-only, in the owner's voice, 145 words: the paste that left `6. 7. 1. 2. 3.`, the same after
Enter / a delete / a moved line, Tab dropping spaces where it should nest "the way Obsidian does",
Shift+Tab to come back out, what is nested under the item going along with it, "only the list I'm
editing should change", and leave `1. 1. 1.` lists alone. It names no file, no function and no rule.
The three stated constraints are the three that are the owner's preference rather than derivable
(other lists are not tidied up; same-number lists are a style; an item carries its children, where
stock Obsidian moves only the line the cursor is on).

## What makes it hard

- Scope: a list model that has to be right about nesting, blank lines, fences, `1.` vs `1)`,
  tab-indented sublists and first numbers, used from five different edits (paste, Enter, delete,
  move line, Tab / Shift+Tab) that all have to agree.
- Nothing says where the hooks go. Paste reaches the buffer through `App::on_paste` ->
  `paste_text` -> `Editor::insert_str`; a moved line through the palette action; keys through
  `App::on_key` -> `Editor::on_key`. The tests drive the app, so a fix on one path only is seen.
- Tab has to produce markdown that is actually nested. The obvious unit is `tab_width` (2), and two
  spaces under `1. a` do not nest: the app's own reading view draws `1. a / 2. b / 3. c`.
- The edit and its renumbering are one undo step, and the cursor has to stay on its text when a
  marker changes width under it (`9.` -> `10.`, `10.` -> `   1.`).

## Checks

All held-out tests are in `hidden/src/app/heldout_lists.rs`, a child module of `app` (declared by
the check's command) so it can open a real session: `App::launch(Launch::File(..))` on a scratch
vault, then `app.on_key`, `app.on_paste`, `app.run_action(Action::MoveLineUp/Down)`, and reads
`app.editor.lines()`. `hidden/heldout/run.sh` points `HOME` and `TMPDIR` inside the run's `{root}`
(the app writes a settings note under `$HOME/.config/catcher`), so nothing outside the run is
touched and parallel graders cannot meet.

| check | module | at base | catches |
| --- | --- | --- | --- |
| existing_tests_pass | the 678 test names in `hidden/heldout/base_tests.txt` must each report ok | pass | breaking or deleting what was there |
| plain_editing_unchanged | `plain` | pass | Tab outside a list, Enter continuing a list, plain pastes, numbered lines pasted into a code fence, moving a plain line |
| same_number_lists_left_alone | `all_ones` | pass | renumbering a `1. 1. 1.` list on paste, Enter, delete or move |
| paste_joins_count | `paste_count` | fail | tail and middle pastes, a list starting at 5, jumbled pasted numbers, a loose list |
| nested_lists_count_separately | `nested` | fail | sublist vs outer list on paste / Enter / delete, tab-indented sublist |
| code_and_other_lists_untouched | `elsewhere` | fail | `1.` lines in a fence inside an item, another (misnumbered) list above or below |
| enter_delete_move_recount | `edits` | fail | Enter mid-list (past 9, loose list, `1)` lists), selection delete with Backspace and Delete, an item rubbed out with Backspace, line and selection moves |
| tab_nests_shift_tab_unnests | `nesting` | fail | nesting that really nests, restart at 1, outer list recounted, children carried, joining an existing sublist, two-digit parent, selection, bullets and tasks, round trip |
| edit_is_one_undo_step | `undo_step` | fail | one ^Z restores the note exactly, one ^Y brings the edit back, for all six edits |
| cursor_stays_on_its_text | `cursor` | fail | where the next typed character lands after paste, Tab, Shift+Tab, move, Enter; non-ASCII text |

Protected: `Cargo.toml`, `Cargo.lock`, `src/testutil.rs`. The existing tests are inline in the
source files a solution has to edit, so they cannot be protected as files; the name list is what
keeps them from being deleted.

## What the tests assume

- List structure is read with `pulldown-cmark` (the crate the reading view uses), not from
  indentation. `shape()` gives one line per item, two spaces per parsed level, with the number the
  file carries. So any indentation that markdown reads as nested passes (3 to 6 spaces or a tab
  under `1. a`); the reference's "parent's text column" is not required. A `helpers` test pins this.
- Exact text is asserted only where no indentation choice is involved (flat lists, files that
  already have their indentation) and for a Tab followed by Shift+Tab, which must give back the
  original.
- Shift+Tab is sent as `KeyCode::BackTab` with `SHIFT`, which is what crossterm delivers and what
  the base's table code already matches on.
- `1. 1. 1.` tests never assert the marker of a new item (the reference writes `2. ` there; `1. `
  is as good), only that the existing items keep their `1.`.
- Undo tests require only that the edit changed the note; cursor tests compare the item's text
  without its marker. Neither depends on the numbers being right, so each check fails for its own
  reason (changed after v1).
- Avoided on purpose, because the reference is arbitrary or wrong there: Tab on the first item of a
  list, a second Tab on an item that cannot go deeper, Shift+Tab on a middle child (Obsidian and
  outliners disagree), moving the first item of a list (the reference counts from the moved item's
  number), cut (^X goes through the system clipboard), a fence at the margin between two lists (the
  reference counts through it), a number typed over by hand, `1.` and `1)` mixed in one run.
- Entry points used all exist at base: `App::launch`, `cli::Launch::File`, `App::on_key`,
  `App::on_paste`, `App::run_action` (private, reached as a child module), `keys::Action`,
  `app.editor` with `lines()`, `text()`, `move_cursor`, `tab_width`.

## Calibration (Sonnet 5.5, low effort, 2 runs each)

### v1: 0.4, 0.4 (both fail; 82 s and 99 s)

- Both: numbered lines pasted into a code fence were renumbered (`plain_editing_unchanged`); a
  loose list was not treated as one list on paste; Tab indented by `tab_width` (2 spaces), which
  does not nest under `1. `, and Shift+Tab on a 3-space sublist left ` 2. y` with the outer list
  not recounted.
- Run 1: items pasted after a sublist did not join the outer list; cursor left behind after a
  paste and a move that changed the marker's width.
- Run 2: Enter in a loose list did not count past the blank line.
- All genuine. But `edit_is_one_undo_step` and `cursor_stays_on_its_text` failed only because
  their tests also asserted the renumbered result (`4. c`, `10. jXkl`), not because undo or the
  cursor were wrong.
- Changed: undo tests now only require that the edit changed the note; cursor tests compare the
  item's text without its marker. Re-grading the v1 patches against the changed tests gives 0.5
  and 0.6.

### v2: 0.6, 0.6 (both fail; 105 s and 125 s). Final.

- Both: loose list on paste (`1. a / 1. x / (blank) / 2. b`); items pasted after a sublist start
  again at 1 (`nested_lists_count_separately`); Tab by `tab_width` again, plus the outer list not
  recounted after Shift+Tab.
- Run 1: cursor wrong after a move and a Tab that changed the marker's width.
- Run 2: Enter in a loose list.
- Nothing changed after v2.

On the Tab miss (4 of 4 runs): the solutions write `1. a\n  1. b\n2. c`. Rendered with the base's
own `render::render`, that is `1. a`, `2. b`, `3. c`: the item is not nested on the page and the
file's numbers disagree with the page's, which is the complaint the prompt opens with. With three
spaces the same call gives `1. a`, `  1. b`, `2. c`. Bullets (two spaces under `- a`) pass.

## Likely wrong fixes and the check that catches each

- Indent by `tab_width`: `tab_nests_shift_tab_unnests`.
- Indent by a fixed 3: fails under `10. jkl` (`nesting_under_a_two_digit_item_really_nests`).
- Move only the item's own line on Tab: `an_item_takes_what_is_nested_under_it_along`.
- Replace a selection with spaces as base does, or move only the cursor's row:
  `every_item_a_selection_touches_moves`.
- A blank line ends the list: `paste_joins_count`, `enter_delete_move_recount`.
- Renumber from 1: `a_list_that_starts_at_five_still_starts_at_five`, the delete and move tests
  that start at 3.
- One flat count for the whole block, or nested items counted with the outer ones:
  `nested_lists_count_separately`.
- Renumber every list in the note: `code_and_other_lists_untouched`.
- No fence check: `plain_editing_unchanged` (paste into a fence), `code_and_other_lists_untouched`.
- Hook only the paste path, or only Enter: `enter_delete_move_recount` (Backspace, Delete, the
  move actions).
- Renumber as a second recorded edit (`set_line` per row from the app): `edit_is_one_undo_step`.
- Leave the cursor column alone when a marker changes width or an item moves in or out:
  `cursor_stays_on_its_text`.
- Byte offsets for the cursor: `after_tab_on_an_item_whose_text_is_not_ascii`.
- Renumber `1. 1. 1.`: `same_number_lists_left_alone`.

## Independent review (after v2)

Blind solve from the prompt and the base alone, about 10 minutes: 9 of 10 checks.
`tab_nests_shift_tab_unnests` failed on 6 of its 10 tests, for two separate reasons.

- Five tests: Tab indented by `tab_width`, the same miss as all four Sonnet runs. Genuine. The
  solver read the CommonMark rule, saw `md::list_depth` call two spaces a level, and went with the
  app's unit without rendering the result. Tests left as they are.
- `an_item_takes_what_is_nested_under_it_along`: the solve moved only the cursor's line, on
  purpose, because that is what stock Obsidian does (carrying the sublist is what the Outliner
  plugin adds), and v1 run 1 chose and reported the same thing. The prompt said only "the way
  Obsidian does", so the test asked for the reference's choice against the prompt's own pointer.
  Repaired in the prompt, not the test: one sentence, "Whatever is already nested under the item
  goes along with it". No test changed, so the v2 rows stand as graded (both runs fail that check
  on `tab_width` regardless).

With the indent changed to the parent's text column and the children carried, the same
independently written solution (a before/after diff of the buffer around each structural edit, in
`Editor`, sharing nothing with the reference's design) passes all 55 held-out tests.
