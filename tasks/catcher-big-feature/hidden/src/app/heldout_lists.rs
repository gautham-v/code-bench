//! Held-out tests: a note open in the app, driven the way the terminal drives
//! it (keys, a bracketed paste, the palette's move-line actions), and what the
//! buffer holds afterwards.
//!
//! List structure is read the way the reading view reads it, with the
//! markdown parser, so nothing here depends on how many spaces (or which
//! whitespace) a nested item was indented with.

use super::App;
use crate::keys::Action;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};

/// A session on a scratch vault holding one note with `body`.
fn open(name: &str, body: &str) -> App {
    // launching reads and writes the settings note; one at a time
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("heldout-lists-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("note.md");
    std::fs::write(&path, body).unwrap();
    let app = App::launch(crate::cli::Launch::File(path)).expect("the app opens the note");
    assert_eq!(app.editor.text(), body, "the note is in the editor as written");
    app
}

fn text(app: &App) -> String {
    app.editor.lines().join("\n")
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(KeyEvent::new(code, mods));
}

fn tab(app: &mut App) {
    press(app, KeyCode::Tab, KeyModifiers::NONE);
}

/// ⇧⇥ as a terminal sends it.
fn shift_tab(app: &mut App) {
    press(app, KeyCode::BackTab, KeyModifiers::SHIFT);
}

fn enter(app: &mut App) {
    press(app, KeyCode::Enter, KeyModifiers::NONE);
}

fn backspace(app: &mut App) {
    press(app, KeyCode::Backspace, KeyModifiers::NONE);
}

fn undo(app: &mut App) {
    press(app, KeyCode::Char('z'), KeyModifiers::CONTROL);
}

fn redo(app: &mut App) {
    press(app, KeyCode::Char('y'), KeyModifiers::CONTROL);
}

fn type_str(app: &mut App, s: &str) {
    for c in s.chars() {
        press(app, KeyCode::Char(c), KeyModifiers::NONE);
    }
}

fn paste(app: &mut App, s: &str) {
    app.on_paste(s.to_string());
}

/// The row of the one line that contains `needle`.
fn row_of(app: &App, needle: &str) -> usize {
    let rows: Vec<usize> = app
        .editor
        .lines()
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains(needle))
        .map(|(r, _)| r)
        .collect();
    assert_eq!(rows.len(), 1, "one line holds {needle:?} in:\n{}", text(app));
    rows[0]
}

/// The line that contains `needle`, without its indentation.
fn line_of(app: &App, needle: &str) -> String {
    app.editor.lines()[row_of(app, needle)].trim().to_string()
}

/// The text of the item on the line that contains `needle`: the line without
/// its indentation and without its `12. ` / `3) ` / `- ` marker.
fn item_text(app: &App, needle: &str) -> String {
    let line = line_of(app, needle);
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    let rest = &line[digits..];
    let body = if digits > 0 {
        rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))
    } else {
        ["- ", "* ", "+ "].iter().find_map(|m| rest.strip_prefix(m))
    };
    body.unwrap_or_else(|| panic!("{line:?} is a list item")).to_string()
}

/// Cursor (no selection) `offset` chars into `needle` on the line holding it.
fn cursor_in(app: &mut App, needle: &str, offset: usize) {
    let row = row_of(app, needle);
    let line = &app.editor.lines()[row];
    let col = line[..line.find(needle).unwrap()].chars().count() + offset;
    app.editor.move_cursor((row, col), false);
}

/// Cursor at the end of the line holding `needle`.
fn cursor_at_end(app: &mut App, needle: &str) {
    let row = row_of(app, needle);
    let len = app.editor.lines()[row].chars().count();
    app.editor.move_cursor((row, len), false);
}

/// Cursor at the very start of the line holding `needle`.
fn cursor_at_start(app: &mut App, needle: &str) {
    let row = row_of(app, needle);
    app.editor.move_cursor((row, 0), false);
}

/// Cursor on the last line of the buffer, which must be empty.
fn cursor_on_last_blank(app: &mut App) {
    let last = app.editor.lines().len() - 1;
    assert_eq!(app.editor.lines()[last], "", "the note ends with an empty line");
    app.editor.move_cursor((last, 0), false);
}

/// Select from `from` to `to` (row, col), the cursor ending at `to`.
fn select(app: &mut App, from: (usize, usize), to: (usize, usize)) {
    app.editor.move_cursor(from, false);
    app.editor.move_cursor(to, true);
}

/// The lists of `md` as the markdown parser sees them: one line per item,
/// two spaces per level of nesting, then the item's own first line as the
/// file has it (so `3. text` shows the number the file carries).
fn shape(md: &str) -> String {
    let mut depth = 0usize;
    let mut out: Vec<String> = Vec::new();
    for (event, range) in Parser::new(md).into_offset_iter() {
        match event {
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Item) => {
                let first = md[range.start..].lines().next().unwrap_or("").trim();
                out.push(format!("{}{}", "  ".repeat(depth - 1), first));
            }
            _ => {}
        }
    }
    out.join("\n")
}

fn assert_shape(app: &App, want: &str) {
    let got = shape(&text(app));
    assert_eq!(
        got,
        want,
        "\nlists, as parsed:\n{got}\n\nwanted:\n{want}\n\nbuffer:\n{}\n",
        text(app)
    );
}

const TEN: &str = "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n10. jkl\n";

mod helpers {
    use super::*;

    // the reader these tests lean on, checked against lists written by hand
    #[test]
    fn shape_reads_nesting_whatever_it_was_indented_with() {
        assert_eq!(shape("1. a\n   1. b\n2. c\n"), "1. a\n  1. b\n2. c");
        assert_eq!(shape("1. a\n    1. b\n2. c\n"), "1. a\n  1. b\n2. c");
        assert_eq!(shape("1. a\n\t1. b\n2. c\n"), "1. a\n  1. b\n2. c");
        assert_eq!(shape("- a\n  - b\n    - c\n- d\n"), "- a\n  - b\n    - c\n- d");
        // two spaces do not nest under `1. `: the parser reads a sibling
        assert_eq!(shape("1. a\n  2. b\n"), "1. a\n2. b");
        assert_eq!(shape("10. j\n    1. k\n"), "10. j\n  1. k");
    }
}

/// What already worked, and must keep working.
mod plain {
    use super::*;

    #[test]
    fn tab_outside_a_list_still_indents_with_spaces() {
        let mut app = open("plain-tab", "plain words\n");
        let width = app.editor.tab_width;
        cursor_in(&mut app, "plain", 0);
        tab(&mut app);
        assert_eq!(text(&app), format!("{}plain words", " ".repeat(width)));
    }

    #[test]
    fn enter_at_the_end_of_an_item_continues_the_list() {
        let mut app = open("plain-enter", "1. one\n2. two\n\n- x\n- y\n");
        cursor_at_end(&mut app, "two");
        enter(&mut app);
        type_str(&mut app, "three");
        assert_eq!(text(&app), "1. one\n2. two\n3. three\n\n- x\n- y");
        cursor_at_end(&mut app, "- y");
        enter(&mut app);
        type_str(&mut app, "z");
        assert_eq!(text(&app), "1. one\n2. two\n3. three\n\n- x\n- y\n- z");
    }

    #[test]
    fn pasted_text_lands_as_it_was_written() {
        let mut app = open("plain-paste", "Some words.\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "- one\n- two\n\nA paragraph, 2. not a list.");
        assert_eq!(
            text(&app),
            "Some words.\n- one\n- two\n\nA paragraph, 2. not a list."
        );
        // and it is still one undo step
        undo(&mut app);
        assert_eq!(text(&app), "Some words.\n");
    }

    #[test]
    fn text_pasted_into_an_item_leaves_the_numbers_alone() {
        let mut app = open("plain-paste-item", "1. one\n2. two\n3. three\n");
        cursor_at_end(&mut app, "two");
        paste(&mut app, " and more");
        assert_eq!(text(&app), "1. one\n2. two and more\n3. three");
    }

    #[test]
    fn numbered_lines_pasted_into_a_code_block_stay_as_pasted() {
        let mut app = open("plain-paste-code", "Run:\n\n```\n\n```\n");
        app.editor.move_cursor((3, 0), false);
        paste(&mut app, "3. third\n3. third again\n1. first");
        assert_eq!(
            text(&app),
            "Run:\n\n```\n3. third\n3. third again\n1. first\n```"
        );
    }

    #[test]
    fn moving_a_plain_line_swaps_it_with_its_neighbour() {
        let mut app = open("plain-move", "alpha\nbeta\ngamma\n");
        cursor_in(&mut app, "gamma", 2);
        app.run_action(Action::MoveLineUp);
        assert_eq!(text(&app), "alpha\ngamma\nbeta");
        app.run_action(Action::MoveLineDown);
        assert_eq!(text(&app), "alpha\nbeta\ngamma");
    }
}

/// A pasted list joins the count of the list it landed in.
mod paste_count {
    use super::*;

    #[test]
    fn five_items_pasted_after_seven_come_out_eight_to_twelve() {
        let note = "# Passions\n\n1. Basketball\n2. Writing\n3. Reading fast\n4. Pull-ups\n\
                    5. Drawing\n6. Public speaking\n7. Articulating\n\n";
        let mut app = open("paste-tail", note);
        cursor_on_last_blank(&mut app);
        paste(
            &mut app,
            "1. Hiking\n2. Chatting\n3. SUNSHINE\n4. Feeling productive\n5. Automating",
        );
        assert_eq!(
            text(&app),
            "# Passions\n\n1. Basketball\n2. Writing\n3. Reading fast\n4. Pull-ups\n\
             5. Drawing\n6. Public speaking\n7. Articulating\n8. Hiking\n9. Chatting\n\
             10. SUNSHINE\n11. Feeling productive\n12. Automating"
        );
    }

    #[test]
    fn items_pasted_into_the_middle_push_the_rest_of_the_list_on() {
        let mut app = open("paste-middle", "1. a\n2. b\n3. c\n4. d\n");
        cursor_at_start(&mut app, "3. c");
        paste(&mut app, "1. x\n2. y\n");
        assert_eq!(text(&app), "1. a\n2. b\n3. x\n4. y\n5. c\n6. d");
    }

    #[test]
    fn a_list_that_starts_at_five_still_starts_at_five() {
        let mut app = open("paste-five", "Steps, continued:\n\n5. e\n6. f\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "1. x\n2. y\n3. z");
        assert_eq!(text(&app), "Steps, continued:\n\n5. e\n6. f\n7. x\n8. y\n9. z");
    }

    #[test]
    fn a_list_with_blank_lines_between_its_items_is_still_one_list() {
        let mut app = open("paste-loose", "1. a\n\n2. b\n\n3. c\n");
        app.editor.move_cursor((1, 0), false);
        paste(&mut app, "1. x\n");
        assert_eq!(text(&app), "1. a\n2. x\n\n3. b\n\n4. c");
    }

    #[test]
    fn pasted_items_out_of_order_are_counted_in_the_order_they_landed() {
        let mut app = open("paste-jumbled", "1. one\n2. two\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "3. three\n1. four\n2. five");
        assert_eq!(text(&app), "1. one\n2. two\n3. three\n4. four\n5. five");
    }
}

/// Nested lists keep counts of their own.
mod nested {
    use super::*;

    const NOTE: &str = "1. a\n   1. x\n   2. y\n2. b\n3. c\n";

    #[test]
    fn items_pasted_into_a_sublist_join_it_and_not_the_list_outside() {
        let mut app = open("nested-paste-inner", NOTE);
        cursor_at_start(&mut app, "2. b");
        paste(&mut app, "   1. p\n   2. q\n");
        assert_eq!(text(&app), "1. a\n   1. x\n   2. y\n   3. p\n   4. q\n2. b\n3. c");
    }

    #[test]
    fn items_pasted_after_a_sublist_join_the_outer_list() {
        let mut app = open("nested-paste-outer", "1. a\n   1. x\n   2. y\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "1. b\n2. c");
        assert_eq!(text(&app), "1. a\n   1. x\n   2. y\n2. b\n3. c");
    }

    #[test]
    fn a_new_item_in_a_sublist_recounts_the_sublist_only() {
        let mut app = open("nested-enter", "1. a\n   1. x\n   2. y\n   3. z\n2. b\n");
        cursor_at_end(&mut app, "1. x");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_shape(&app, "1. a\n  1. x\n  2. new\n  3. y\n  4. z\n2. b");
    }

    #[test]
    fn an_item_deleted_from_a_sublist_recounts_the_sublist_only() {
        let mut app = open("nested-delete", "1. a\n   1. x\n   2. y\n   3. z\n2. b\n3. c\n");
        let y = row_of(&app, "2. y");
        select(&mut app, (y, 0), (y + 1, 0));
        backspace(&mut app);
        assert_eq!(text(&app), "1. a\n   1. x\n   2. z\n2. b\n3. c");
    }

    #[test]
    fn a_sublist_indented_with_a_tab_counts_the_same_way() {
        let mut app = open("nested-tab-char", "1. a\n\t1. x\n\t2. y\n\t3. z\n2. b\n3. c\n");
        let y = row_of(&app, "2. y");
        select(&mut app, (y, 0), (y + 1, 0));
        backspace(&mut app);
        assert_eq!(text(&app), "1. a\n\t1. x\n\t2. z\n2. b\n3. c");
        cursor_at_end(&mut app, "2. b");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(text(&app), "1. a\n\t1. x\n\t2. z\n2. b\n3. new\n4. c");
    }

    #[test]
    fn an_item_added_to_the_outer_list_leaves_the_sublist_alone() {
        let mut app = open("nested-outer-enter", "1. a\n2. b\n   1. x\n   2. y\n3. c\n4. d\n");
        cursor_at_end(&mut app, "3. c");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(text(&app), "1. a\n2. b\n   1. x\n   2. y\n3. c\n4. new\n5. d");
    }
}

/// Only the list that was edited changes: code and other lists are not its
/// business.
mod elsewhere {
    use super::*;

    const NOTE: &str = "# Plan\n\n1. draft\n3. review\n7. ship\n\nSome text in between.\n\n\
                        1. install\n2. run this:\n   ```\n   1. not a step\n   1. also not a step\n   ```\n\n";

    #[test]
    fn a_paste_leaves_fenced_code_and_the_other_list_as_they_were() {
        let mut app = open("elsewhere-paste", NOTE);
        cursor_on_last_blank(&mut app);
        paste(&mut app, "1. verify\n2. celebrate");
        assert_eq!(
            text(&app),
            "# Plan\n\n1. draft\n3. review\n7. ship\n\nSome text in between.\n\n\
             1. install\n2. run this:\n   ```\n   1. not a step\n   1. also not a step\n   ```\n\
             3. verify\n4. celebrate"
        );
    }

    #[test]
    fn a_new_item_leaves_fenced_code_and_the_other_list_as_they_were() {
        let mut app = open("elsewhere-enter", NOTE);
        cursor_at_end(&mut app, "1. install");
        enter(&mut app);
        type_str(&mut app, "configure");
        assert_eq!(
            text(&app),
            "# Plan\n\n1. draft\n3. review\n7. ship\n\nSome text in between.\n\n\
             1. install\n2. configure\n3. run this:\n   ```\n   1. not a step\n   1. also not a step\n   ```\n"
        );
    }

    #[test]
    fn a_list_further_down_the_note_is_not_recounted_either() {
        let mut app = open(
            "elsewhere-below",
            "1. a\n2. b\n3. c\n\nNotes:\n\n2. second thing first\n5. fifth\n",
        );
        let b = row_of(&app, "2. b");
        select(&mut app, (b, 0), (b + 1, 0));
        backspace(&mut app);
        assert_eq!(
            text(&app),
            "1. a\n2. c\n\nNotes:\n\n2. second thing first\n5. fifth"
        );
    }
}

/// A list numbered `1. 1. 1.` on purpose stays that way.
mod all_ones {
    use super::*;

    #[test]
    fn a_paste_into_it_changes_no_number() {
        let mut app = open("ones-paste", "1. a\n1. b\n1. c\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "1. x\n1. y");
        assert_eq!(text(&app), "1. a\n1. b\n1. c\n1. x\n1. y");
    }

    #[test]
    fn a_new_item_in_it_leaves_the_others_at_one() {
        let mut app = open("ones-enter", "1. a\n1. b\n1. c\n1. d\n");
        cursor_at_end(&mut app, "1. b");
        enter(&mut app);
        type_str(&mut app, "new");
        let lines: Vec<String> = app.editor.lines().to_vec();
        assert_eq!(lines.len(), 5, "{lines:?}");
        assert_eq!(
            [&lines[0], &lines[1], &lines[3], &lines[4]],
            ["1. a", "1. b", "1. c", "1. d"],
            "{lines:?}"
        );
        assert!(lines[2].ends_with(". new"), "{lines:?}");
    }

    #[test]
    fn deleting_and_moving_items_in_it_changes_no_number() {
        let mut app = open("ones-delete", "1. a\n1. b\n1. c\n1. d\n");
        select(&mut app, (1, 0), (2, 0));
        backspace(&mut app);
        assert_eq!(text(&app), "1. a\n1. c\n1. d");
        cursor_in(&mut app, "1. d", 3);
        app.run_action(Action::MoveLineUp);
        assert_eq!(text(&app), "1. a\n1. d\n1. c");
    }
}

/// ⏎, a delete and a moved line leave the list counting straight.
mod edits {
    use super::*;

    #[test]
    fn enter_in_the_middle_counts_the_rest_on() {
        let mut app = open("edits-enter", "1. a\n2. b\n3. c\n4. d\n");
        cursor_at_end(&mut app, "2. b");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(text(&app), "1. a\n2. b\n3. new\n4. c\n5. d");
    }

    #[test]
    fn enter_counts_on_past_nine() {
        let mut app = open("edits-enter-ten", "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n");
        cursor_at_end(&mut app, "3. c");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(
            text(&app),
            "1. a\n2. b\n3. c\n4. new\n5. d\n6. e\n7. f\n8. g\n9. h\n10. i"
        );
    }

    #[test]
    fn enter_in_a_list_with_blank_lines_between_its_items_counts_past_them() {
        let mut app = open("edits-enter-loose", "1. a\n\n2. b\n\n3. c\n");
        cursor_at_end(&mut app, "1. a");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(text(&app), "1. a\n2. new\n\n3. b\n\n4. c");
    }

    #[test]
    fn a_list_numbered_with_parentheses_counts_the_same_way() {
        let mut app = open("edits-paren", "1) a\n2) b\n3) c\n4) d\n");
        cursor_at_end(&mut app, "2) b");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(text(&app), "1) a\n2) b\n3) new\n4) c\n5) d");
        select(&mut app, (0, 4), (1, 4));
        backspace(&mut app);
        assert_eq!(text(&app), "1) a\n2) new\n3) c\n4) d");
    }

    #[test]
    fn a_selected_item_deleted_closes_the_gap() {
        let mut app = open("edits-delete", "1. a\n2. b\n3. c\n4. d\n");
        select(&mut app, (1, 0), (2, 0));
        backspace(&mut app);
        assert_eq!(text(&app), "1. a\n2. c\n3. d");
        // and the forward delete key does the same
        let mut app = open("edits-delete-fwd", "3. a\n4. b\n5. c\n6. d\n");
        select(&mut app, (1, 0), (3, 0));
        press(&mut app, KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(text(&app), "3. a\n4. d");
    }

    #[test]
    fn an_item_backspaced_away_closes_the_gap() {
        // rub out an empty item, marker and all, and then the line it left
        let mut app = open("edits-rubout", "1. a\n2. \n3. c\n4. d\n");
        app.editor.move_cursor((1, 3), false);
        for _ in 0..4 {
            backspace(&mut app);
        }
        assert_eq!(text(&app), "1. a\n2. c\n3. d");
    }

    #[test]
    fn a_moved_item_takes_the_number_of_the_place_it_moved_to() {
        let mut app = open("edits-move", "1. a\n2. b\n3. c\n4. d\n");
        cursor_in(&mut app, "3. c", 3);
        app.run_action(Action::MoveLineUp);
        assert_eq!(text(&app), "1. a\n2. c\n3. b\n4. d");
        app.run_action(Action::MoveLineDown);
        app.run_action(Action::MoveLineDown);
        assert_eq!(text(&app), "1. a\n2. b\n3. d\n4. c");
    }

    #[test]
    fn a_moved_selection_is_recounted_with_the_items_it_passed() {
        let mut app = open("edits-move-sel", "3. a\n4. b\n5. c\n6. d\n");
        select(&mut app, (1, 1), (2, 2));
        app.run_action(Action::MoveLineDown);
        assert_eq!(text(&app), "3. a\n4. d\n5. b\n6. c");
    }
}

/// ⇥ nests an item under the one above it, ⇧⇥ brings it back out, and the
/// numbers follow.
mod nesting {
    use super::*;

    #[test]
    fn tab_nests_the_item_and_both_lists_count_right() {
        let mut app = open("nest-tab", "1. a\n2. b\n3. c\n");
        cursor_in(&mut app, "2. b", 4);
        tab(&mut app);
        assert_shape(&app, "1. a\n  1. b\n2. c");
        // and back out again
        shift_tab(&mut app);
        assert_eq!(text(&app), "1. a\n2. b\n3. c");
    }

    #[test]
    fn tab_works_from_anywhere_on_the_line() {
        for (name, col) in [("start", 0), ("marker", 2), ("text", 5)] {
            let mut app = open(&format!("nest-anywhere-{name}"), "1. a\n2. bcd\n3. e\n");
            app.editor.move_cursor((1, col), false);
            tab(&mut app);
            assert_shape(&app, "1. a\n  1. bcd\n2. e");
        }
    }

    #[test]
    fn a_bullet_nests_and_comes_back() {
        let mut app = open("nest-bullet", "- a\n- b\n- c\n");
        cursor_in(&mut app, "- c", 3);
        tab(&mut app);
        assert_shape(&app, "- a\n- b\n  - c");
        shift_tab(&mut app);
        assert_eq!(text(&app), "- a\n- b\n- c");
    }

    #[test]
    fn a_task_nests_under_the_task_above_it() {
        let mut app = open("nest-task", "- [ ] a\n- [x] b\n- [ ] c\n");
        cursor_in(&mut app, "[x] b", 5);
        tab(&mut app);
        assert_shape(&app, "- [ ] a\n  - [x] b\n- [ ] c");
        shift_tab(&mut app);
        assert_eq!(text(&app), "- [ ] a\n- [x] b\n- [ ] c");
    }

    #[test]
    fn an_item_takes_what_is_nested_under_it_along() {
        let note = "1. a\n2. b\n   1. x\n   2. y\n3. c\n";
        let mut app = open("nest-children", note);
        cursor_in(&mut app, "2. b", 4);
        tab(&mut app);
        assert_shape(&app, "1. a\n  1. b\n    1. x\n    2. y\n2. c");
        cursor_in(&mut app, "1. b", 4);
        shift_tab(&mut app);
        assert_shape(&app, "1. a\n2. b\n  1. x\n  2. y\n3. c");
    }

    #[test]
    fn an_item_nested_after_a_sublist_joins_its_count() {
        let mut app = open("nest-join", "1. a\n   1. x\n   2. y\n2. b\n3. c\n");
        cursor_in(&mut app, "2. b", 4);
        tab(&mut app);
        assert_shape(&app, "1. a\n  1. x\n  2. y\n  3. b\n2. c");
    }

    #[test]
    fn the_last_item_of_a_sublist_comes_out_into_the_outer_count() {
        let mut app = open("nest-out-last", "1. a\n   1. x\n   2. y\n2. b\n3. c\n");
        cursor_in(&mut app, "2. y", 4);
        shift_tab(&mut app);
        assert_shape(&app, "1. a\n  1. x\n2. y\n3. b\n4. c");
    }

    #[test]
    fn nesting_under_a_two_digit_item_really_nests() {
        let mut app = open("nest-ten", &format!("{TEN}11. k\n12. l\n"));
        cursor_in(&mut app, "11. k", 5);
        tab(&mut app);
        assert_shape(
            &app,
            "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n10. jkl\n  1. k\n11. l",
        );
    }

    #[test]
    fn every_item_a_selection_touches_moves() {
        let mut app = open("nest-selection", "1. a\n2. b\n3. c\n4. d\n");
        select(&mut app, (1, 0), (2, 4));
        tab(&mut app);
        assert_shape(&app, "1. a\n  1. b\n  2. c\n2. d");
        let b = row_of(&app, ". b");
        let c_len = app.editor.lines()[b + 1].chars().count();
        select(&mut app, (b, 0), (b + 1, c_len));
        shift_tab(&mut app);
        assert_eq!(text(&app), "1. a\n2. b\n3. c\n4. d");
    }

    #[test]
    fn an_item_at_the_margin_has_nowhere_further_out_to_go() {
        let mut app = open("nest-margin", "1. a\n2. b\n\n- x\n- y\n");
        cursor_in(&mut app, "2. b", 4);
        shift_tab(&mut app);
        cursor_in(&mut app, "- y", 3);
        shift_tab(&mut app);
        assert_eq!(text(&app), "1. a\n2. b\n\n- x\n- y");
    }
}

/// Each of these is one edit as far as undo is concerned.
mod undo_step {
    use super::*;

    /// One undo puts back `before`; one redo puts back what the edit made.
    fn one_step(app: &mut App, before: &str, what: &str) {
        let after = text(app);
        assert_ne!(after, before.trim_end_matches('\n'), "{what} changed the note");
        undo(app);
        assert_eq!(
            text(app),
            before.trim_end_matches('\n'),
            "one undo takes back {what}, renumbering and all"
        );
        redo(app);
        assert_eq!(text(app), after, "one redo brings {what} back whole");
    }

    #[test]
    fn a_paste_and_its_renumbering() {
        let before = "1. a\n2. b\n3. c\n4. d\n";
        let mut app = open("undo-paste", before);
        cursor_at_start(&mut app, "3. c");
        paste(&mut app, "1. x\n2. y\n");
        one_step(&mut app, before, "the paste");
    }

    #[test]
    fn an_enter_and_its_renumbering() {
        let before = "1. a\n2. b\n3. c\n4. d\n";
        let mut app = open("undo-enter", before);
        cursor_at_end(&mut app, "2. b");
        enter(&mut app);
        one_step(&mut app, before, "the new item");
    }

    #[test]
    fn a_delete_and_its_renumbering() {
        let before = "1. a\n2. b\n3. c\n4. d\n";
        let mut app = open("undo-delete", before);
        select(&mut app, (1, 0), (2, 0));
        backspace(&mut app);
        one_step(&mut app, before, "the delete");
    }

    #[test]
    fn a_move_and_its_renumbering() {
        let before = "1. a\n2. b\n3. c\n4. d\n";
        let mut app = open("undo-move", before);
        cursor_in(&mut app, "3. c", 3);
        app.run_action(Action::MoveLineUp);
        one_step(&mut app, before, "the move");
    }

    #[test]
    fn a_tab_with_everything_it_carried() {
        let before = "1. a\n2. b\n   1. x\n   2. y\n3. c\n";
        let mut app = open("undo-tab", before);
        cursor_in(&mut app, "2. b", 4);
        tab(&mut app);
        one_step(&mut app, before, "the tab");
    }

    #[test]
    fn a_shift_tab_with_its_renumbering() {
        let before = "1. a\n   1. x\n   2. y\n2. b\n3. c\n";
        let mut app = open("undo-shift-tab", before);
        cursor_in(&mut app, "2. y", 4);
        shift_tab(&mut app);
        one_step(&mut app, before, "the shift-tab");
    }
}

/// The cursor stays on the text it was on while markers move and change
/// width under it; what is typed next says where it is.
mod cursor {
    use super::*;

    #[test]
    fn after_a_paste_whose_marker_grew() {
        let mut app = open("cursor-paste", "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n\n");
        cursor_on_last_blank(&mut app);
        paste(&mut app, "1. ten");
        type_str(&mut app, "!");
        assert_eq!(item_text(&app, "ten"), "ten!");
    }

    #[test]
    fn after_tab_on_a_bullet() {
        let mut app = open("cursor-tab-bullet", "- a\n- bcd\n");
        cursor_in(&mut app, "bcd", 1);
        tab(&mut app);
        type_str(&mut app, "X");
        assert_eq!(item_text(&app, "cd"), "bXcd");
    }

    #[test]
    fn after_tab_on_an_item_whose_text_is_not_ascii() {
        let mut app = open("cursor-tab-unicode", "1. caf\u{e9}\n2. na\u{ef}ve \u{65e5}\u{672c}\u{8a9e} r\u{e9}sum\u{e9}\n3. zed\n");
        cursor_in(&mut app, "r\u{e9}sum\u{e9}", 2);
        tab(&mut app);
        type_str(&mut app, "X");
        assert_eq!(
            item_text(&app, "sum"),
            "na\u{ef}ve \u{65e5}\u{672c}\u{8a9e} r\u{e9}Xsum\u{e9}"
        );
    }

    #[test]
    fn after_tab_on_an_item_whose_marker_shrank() {
        let mut app = open("cursor-tab-ten", TEN);
        cursor_in(&mut app, "jkl", 1);
        tab(&mut app);
        type_str(&mut app, "X");
        assert_eq!(item_text(&app, "kl"), "jXkl");
    }

    #[test]
    fn after_shift_tab_on_an_item_whose_marker_grew() {
        let note = "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n   1. jkl\n";
        let mut app = open("cursor-shift-tab", note);
        cursor_in(&mut app, "jkl", 1);
        shift_tab(&mut app);
        type_str(&mut app, "X");
        assert_eq!(item_text(&app, "kl"), "jXkl");
    }

    #[test]
    fn after_a_move_that_changed_the_marker_s_width() {
        let mut app = open("cursor-move", TEN);
        cursor_in(&mut app, "jkl", 1);
        app.run_action(Action::MoveLineUp);
        type_str(&mut app, "X");
        assert_eq!(item_text(&app, "kl"), "jXkl");
        assert_eq!(item_text(&app, ". i"), "i");
    }

    #[test]
    fn after_enter_the_new_item_is_where_typing_goes() {
        let mut app = open("cursor-enter", "1. a\n2. b\n3. c\n4. d\n5. e\n6. f\n7. g\n8. h\n9. i\n");
        cursor_at_end(&mut app, "8. h");
        enter(&mut app);
        type_str(&mut app, "new");
        assert_eq!(item_text(&app, "new"), "new");
        assert_eq!(item_text(&app, ". i"), "i");
    }
}
