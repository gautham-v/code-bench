//! Held-out tests. Each drives the app the way a session does — keys in,
//! the drawn page and the public state out — against a vault and a home
//! directory of its own under the build directory.

use crate::app::{App, Item, Overlay, View};
use crate::cli::Launch;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// HOME and CATCHER_DIR are process-wide, so one test at a time.
static LOCK: Mutex<()> = Mutex::new(());

struct Bed {
    root: PathBuf,
    home: PathBuf,
    vault: PathBuf,
    _guard: MutexGuard<'static, ()>,
}

fn bed(name: &str) -> Bed {
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base
        .join("heldout-tmp")
        .join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("home/.config/catcher")).unwrap();
    fs::create_dir_all(root.join("vault")).unwrap();
    let root = fs::canonicalize(&root).unwrap();
    let (home, vault) = (root.join("home"), root.join("vault"));
    std::env::set_var("HOME", &home);
    std::env::remove_var("CATCHER_DIR");
    std::env::remove_var("TINYNOTE_DIR");
    fs::write(
        home.join(".config/catcher/settings.md"),
        format!("- notes_dir: {}\n- window_title: off\n", vault.display()),
    )
    .unwrap();
    Bed {
        root,
        home,
        vault,
        _guard: guard,
    }
}

impl Bed {
    fn write(&self, rel: &str, body: &str) -> PathBuf {
        let path = self.vault.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, body).unwrap();
        path
    }

    fn open(&self, rel: &str) -> App {
        let app = App::launch(Launch::File(self.vault.join(rel))).expect("the app launches");
        assert_eq!(
            app.active_note().path,
            self.vault.join(rel),
            "precondition: the session opens on {rel}"
        );
        app
    }
}

fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(key(code, mods));
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        press(app, KeyCode::Char(c), KeyModifiers::NONE);
    }
}

fn enter(app: &mut App) {
    press(app, KeyCode::Enter, KeyModifiers::NONE);
}

/// Draw one frame and hand back the source lines the editor put on screen.
fn shown(app: &mut App) -> Vec<String> {
    let mut term = Terminal::new(TestBackend::new(90, 40)).unwrap();
    term.draw(|f| crate::ui::draw(f, app)).unwrap();
    let mut rows: Vec<usize> = app.edit_rows.iter().map(|r| r.line).collect();
    rows.dedup();
    rows.iter()
        .map(|&r| app.editor.lines()[r].clone())
        .collect()
}

/// Draw one frame and hand back the whole screen, one cell per char.
fn screen(app: &mut App) -> Vec<Vec<char>> {
    let mut term = Terminal::new(TestBackend::new(90, 40)).unwrap();
    term.draw(|f| crate::ui::draw(f, app)).unwrap();
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect()
        })
        .collect()
}

/// Where `needle` is drawn: (column, row) of its first cell.
fn find_on(screen: &[Vec<char>], needle: &str) -> Option<(u16, u16)> {
    let want: Vec<char> = needle.chars().collect();
    screen.iter().enumerate().find_map(|(y, row)| {
        row.windows(want.len())
            .position(|w| w == want.as_slice())
            .map(|x| (x as u16, y as u16))
    })
}

fn drawn(app: &mut App, needle: &str) -> bool {
    find_on(&screen(app), needle).is_some()
}

/// A plain left click: the button goes down and comes up on one cell.
fn click(app: &mut App, (x, y): (u16, u16)) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        });
    }
}

/// ^P into the reading view.
fn read(app: &mut App) {
    press(app, KeyCode::Char('p'), KeyModifiers::CONTROL);
    assert!(app.view == View::Preview, "precondition: ^P opens the reading view");
}

fn on_screen(app: &mut App, line: &str) -> bool {
    shown(app).iter().any(|l| l == line)
}

/// Is the cursor's own line one of the lines the last frame drew?
fn cursor_on_screen(app: &mut App) -> bool {
    shown(app);
    let row = app.editor.cursor.0;
    app.edit_rows.iter().any(|r| r.line == row)
}

/// The heading lines the open note has folded, by line number.
fn folded(app: &App) -> Vec<usize> {
    (0..app.editor.lines().len())
        .filter(|&r| app.folded_here(r))
        .collect()
}

/// ⌥← on the heading at `row`, then the cursor back to the top.
fn fold(app: &mut App, row: usize) {
    app.editor.set_cursor((row, 0));
    press(app, KeyCode::Left, KeyModifiers::ALT);
    assert!(
        app.folded_here(row),
        "precondition: alt+left folds the heading on line {row}"
    );
    app.editor.set_cursor((0, 0));
}

/// Another program writes `text` over the open note, and the app is given
/// time to notice. The length always differs from what was there.
fn outside_edit(app: &mut App, path: &Path, text: &str) {
    assert_ne!(fs::read_to_string(path).unwrap().len(), text.len());
    fs::write(path, text).unwrap();
    for _ in 0..60 {
        std::thread::sleep(std::time::Duration::from_millis(150));
        app.tick();
        if app.editor.text() == text {
            return;
        }
    }
    panic!("precondition: an outside edit is picked up from disk");
}

const NOTE: &str = "# Plan\nintro\n## One\na1\na2\n## Two\nb1\nb2\n## Three\nc1\n";

fn palette(app: &mut App, command: &str) {
    press(app, KeyCode::Char('k'), KeyModifiers::CONTROL);
    assert!(app.overlay == Overlay::Palette, "precondition: ^K opens the palette");
    type_text(app, command);
    enter(app);
}

/// ⇧^F, the query, then Enter on the first hit row for line `text`.
fn search_and_open(app: &mut App, query: &str) -> bool {
    press(
        app,
        KeyCode::Char('F'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert!(
        app.overlay == Overlay::QuickOpen && app.contents,
        "precondition: shift+ctrl+F opens search in all files"
    );
    type_text(app, query);
    let items = app.overlay_items();
    let Some(at) = items.iter().position(|i| matches!(i, Item::Line(..))) else {
        return false;
    };
    for _ in 0..at {
        press(app, KeyCode::Down, KeyModifiers::NONE);
    }
    assert_eq!(app.selected, at, "precondition: the hit row is selected");
    enter(app);
    true
}

fn cursor_line(app: &App) -> String {
    app.editor.lines()[app.editor.cursor.0].clone()
}

// ---------------------------------------------------------------------------

/// Another program edits a note that has folded sections.
mod outside_edit_folds {
    use super::*;

    #[test]
    fn folds_follow_their_headings_when_lines_are_added_above() {
        let bed = bed("reload-follow");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        fold(&mut app, 8);
        outside_edit(
            &mut app,
            &path,
            "# Plan\nintro\nnew1\nnew2\n## One\na1\na2\n## Two\nb1\nb2\n## Three\nc1\n",
        );
        assert_eq!(folded(&app), vec![4, 10], "One and Three are still the folded sections");
        let page = shown(&mut app);
        for line in ["new1", "new2", "## One", "## Two", "b1", "b2", "## Three"] {
            assert!(page.iter().any(|l| l == line), "{line:?} is on screen: {page:?}");
        }
        for line in ["a1", "a2", "c1"] {
            assert!(!page.iter().any(|l| l == line), "{line:?} stays folded away: {page:?}");
        }
    }

    #[test]
    fn a_fold_is_not_left_on_whatever_heading_took_its_line() {
        let bed = bed("reload-took");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 5);
        // section One is cut: Two moves up to line 2, Three lands on line 5
        outside_edit(&mut app, &path, "# Plan\nintro\n## Two\nb1\nb2\n## Three\nc1\n");
        assert_eq!(folded(&app), vec![2], "Two is the folded section, not Three");
        let page = shown(&mut app);
        assert!(page.iter().any(|l| l == "c1"), "Three is open: {page:?}");
        assert!(!page.iter().any(|l| l == "b1" || l == "b2"), "Two is folded: {page:?}");
    }

    #[test]
    fn a_fold_whose_heading_is_gone_hides_nothing() {
        let bed = bed("reload-gone");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        fold(&mut app, 8);
        // the "## One" line itself is deleted; its lines join the intro
        outside_edit(
            &mut app,
            &path,
            "# Plan\nintro\na1\na2\n## Two\nb1\nb2\n## Three\nc1\n",
        );
        assert_eq!(folded(&app), vec![7], "only Three is left folded");
        let page = shown(&mut app);
        for line in ["a1", "a2", "## Two", "b1", "b2", "## Three"] {
            assert!(page.iter().any(|l| l == line), "{line:?} is on screen: {page:?}");
        }
        assert!(!page.iter().any(|l| l == "c1"), "Three stays folded: {page:?}");
    }

    #[test]
    fn folds_follow_when_the_outside_edit_drops_unsaved_typing() {
        let bed = bed("reload-dirty");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 5);
        // typed but not yet saved when the other program writes the file
        app.editor.set_cursor((1, 5));
        type_text(&mut app, " more");
        let new = "# Plan\nintro\nextra\n## One\na1\na2\n## Two\nb1\nb2\n## Three\nc1\n";
        fs::write(&path, new).unwrap();
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(app.editor.text(), new, "precondition: the disk wins over the buffer");
        assert_eq!(folded(&app), vec![6], "Two is still the folded section");
        let page = shown(&mut app);
        assert!(page.iter().any(|l| l == "a1"), "One is open: {page:?}");
        assert!(!page.iter().any(|l| l == "b1" || l == "b2"), "Two is folded: {page:?}");
    }
}

/// Where the cursor is left when the note changes under it.
mod outside_edit_cursor {
    use super::*;

    #[test]
    fn the_cursor_is_not_left_inside_a_fold_that_moved_onto_its_line() {
        let bed = bed("reload-cursor");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        fold(&mut app, 8);
        app.editor.set_cursor((7, 1));
        // two lines above go: One is at 1, Three at 6, and line 7 is now c1,
        // which Three's fold hides
        outside_edit(&mut app, &path, "# Plan\n## One\na1\n## Two\nb1\nb2\n## Three\nc1\n");
        assert!(app.folded_here(1), "One is still folded");
        assert!(!on_screen(&mut app, "a1"), "and its section is not drawn");
        let row = app.editor.cursor.0;
        assert!(
            !app.visible.is_hidden(row),
            "the cursor is on line {row}, which a fold hides"
        );
        assert!(cursor_on_screen(&mut app), "the cursor's line {row} is not drawn");
    }

    #[test]
    fn the_cursor_is_not_left_inside_a_fold_after_a_save_that_lost_to_the_disk() {
        let bed = bed("reload-cursor-dirty");
        let path = bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        fold(&mut app, 5);
        app.editor.set_cursor((9, 2));
        type_text(&mut app, "!");
        // the top two lines go and Two's section grows: One is at 0, Two at
        // 3, and line 9 is now b6, which Two's fold hides
        let new = "## One\na1\na2\n## Two\nb1\nb2\nb3\nb4\nb5\nb6\n## Three\nc1\n";
        fs::write(&path, new).unwrap();
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(app.editor.text(), new, "precondition: the disk wins over the buffer");
        assert!(app.folded_here(0), "One is still folded");
        assert!(!on_screen(&mut app, "a2"), "and its section is not drawn");
        let row = app.editor.cursor.0;
        assert!(
            !app.visible.is_hidden(row),
            "the cursor is on line {row}, which a fold hides"
        );
        assert!(cursor_on_screen(&mut app), "the cursor's line {row} is not drawn");
    }
}

/// The note's file gets a new name or folder: from the palette, or from a
/// save that makes the filename follow the title.
mod file_moves_folds {
    use super::*;

    #[test]
    fn folds_survive_rename_file() {
        let bed = bed("rename");
        bed.write("plan.md", NOTE);
        let mut app = bed.open("plan.md");
        fold(&mut app, 5);
        palette(&mut app, "rename file");
        assert!(app.overlay == Overlay::RenameFile, "precondition: the rename prompt opens");
        press(&mut app, KeyCode::Backspace, KeyModifiers::SUPER);
        type_text(&mut app, "roadmap");
        enter(&mut app);
        assert_eq!(
            app.active_note().path,
            bed.vault.join("roadmap.md"),
            "precondition: the file is renamed"
        );
        assert_eq!(folded(&app), vec![5], "Two is still folded after the rename");
        // and stays folded once the page is next rebuilt
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        type_text(&mut app, "x");
        let page = shown(&mut app);
        assert!(!page.iter().any(|l| l == "b1" || l == "b2"), "Two is folded: {page:?}");
        assert!(page.iter().any(|l| l == "## Three"), "{page:?}");
    }

    #[test]
    fn folds_survive_move_to_folder() {
        let bed = bed("move");
        bed.write("plan.md", NOTE);
        fs::create_dir_all(bed.vault.join("archive")).unwrap();
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        fold(&mut app, 8);
        palette(&mut app, "move to folder");
        assert!(app.overlay == Overlay::MoveFile, "precondition: the move picker opens");
        type_text(&mut app, "archive");
        enter(&mut app);
        assert_eq!(
            app.active_note().path,
            bed.vault.join("archive/plan.md"),
            "precondition: the file is moved"
        );
        assert_eq!(folded(&app), vec![2, 8], "One and Three are still folded after the move");
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        type_text(&mut app, "x");
        let page = shown(&mut app);
        assert!(!page.iter().any(|l| l == "a1" || l == "c1"), "{page:?}");
        assert!(page.iter().any(|l| l == "b1"), "{page:?}");
    }


    #[test]
    fn folds_survive_a_save_that_renames_the_file_after_its_title() {
        let bed = bed("title");
        let old = format!("{}.md", crate::notes::slug("Plan"));
        let new = format!("{}.md", crate::notes::slug("Plans"));
        bed.write(&old, NOTE);
        let mut app = bed.open(&old);
        fold(&mut app, 5);
        fold(&mut app, 8);
        // the title gains a letter, and ^S renames the file to match it
        app.editor.set_cursor((0, 6));
        type_text(&mut app, "s");
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_eq!(
            app.active_note().path,
            bed.vault.join(&new),
            "precondition: the file follows its title"
        );
        assert_eq!(folded(&app), vec![5, 8], "Two and Three are still folded");
        app.editor.set_cursor((1, 0));
        type_text(&mut app, "x");
        let page = shown(&mut app);
        assert!(!page.iter().any(|l| l == "b1" || l == "c1"), "{page:?}");
        assert!(page.iter().any(|l| l == "a1"), "{page:?}");
    }

    #[test]
    fn folds_survive_an_autosave_that_renames_the_file_after_its_title() {
        let bed = bed("title-auto");
        let old = format!("{}.md", crate::notes::slug("Plan"));
        let new = format!("{}.md", crate::notes::slug("Plan B"));
        bed.write(&old, NOTE);
        let mut app = bed.open(&old);
        fold(&mut app, 2);
        app.editor.set_cursor((0, 6));
        type_text(&mut app, " B");
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(150));
            app.tick();
            if app.active_note().path == bed.vault.join(&new) {
                break;
            }
        }
        assert_eq!(
            app.active_note().path,
            bed.vault.join(&new),
            "precondition: the autosave renames the file after its title"
        );
        assert_eq!(folded(&app), vec![2], "One is still folded");
        let page = shown(&mut app);
        assert!(!page.iter().any(|l| l == "a1" || l == "a2"), "{page:?}");
    }
}

/// Enter on a search-in-all-files hit that sits inside a folded section.
mod search_hit_in_fold {
    use super::*;

    const ZOO: &str = "# Zoo\nintro\n## Cats\nlion\ntiger\n## Horses\nzebra stripes\npony\n## Birds\nowl\n";

    #[test]
    fn a_hit_inside_a_fold_is_on_screen_under_the_cursor() {
        let bed = bed("hit-fold");
        bed.write("zoo.md", ZOO);
        let mut app = bed.open("zoo.md");
        fold(&mut app, 5);
        assert!(search_and_open(&mut app, "zebra"), "precondition: the search finds the line");
        assert!(app.overlay == Overlay::None);
        assert_eq!(cursor_line(&app), "zebra stripes", "the cursor is on the hit");
        let row = app.editor.cursor.0;
        assert!(!app.visible.is_hidden(row), "the hit's line is still folded away");
        assert!(cursor_on_screen(&mut app), "the hit's line is not drawn");
        assert!(on_screen(&mut app, "zebra stripes"));
    }

    #[test]
    fn a_hit_under_two_folds_is_on_screen_under_the_cursor() {
        let bed = bed("hit-nested");
        bed.write(
            "zoo.md",
            "# Zoo\nintro\n## Horses\nabout horses\n### Wild\nzebra stripes\nonager\n## Birds\nowl\n",
        );
        let mut app = bed.open("zoo.md");
        // the inner section first, then the one that holds it
        fold(&mut app, 4);
        fold(&mut app, 2);
        assert!(search_and_open(&mut app, "zebra"), "precondition: the search finds the line");
        assert_eq!(cursor_line(&app), "zebra stripes", "the cursor is on the hit");
        let row = app.editor.cursor.0;
        assert!(!app.visible.is_hidden(row), "the hit's line is still folded away");
        assert!(cursor_on_screen(&mut app), "the hit's line is not drawn");
    }

    #[test]
    fn a_hit_inside_a_fold_of_another_open_note_is_on_screen_too() {
        let bed = bed("hit-fold-other");
        bed.write("zoo.md", ZOO);
        let other = bed.write("other.md", "# Other\nnothing here\n");
        let mut app = bed.open("zoo.md");
        fold(&mut app, 2);
        fold(&mut app, 5);
        app.open_path(&other);
        assert_eq!(app.active_note().path, other, "precondition: the other note opens");
        assert!(search_and_open(&mut app, "tiger"), "precondition: the search finds the line");
        assert_eq!(app.active_note().path, bed.vault.join("zoo.md"));
        assert_eq!(cursor_line(&app), "tiger", "the cursor is on the hit");
        let row = app.editor.cursor.0;
        assert!(!app.visible.is_hidden(row), "the hit's line is still folded away");
        assert!(cursor_on_screen(&mut app), "the hit's line is not drawn");
    }
}

/// Enter on a hit in a note that has been typed in since it was last saved.
mod search_hit_unsaved {
    use super::*;

    #[test]
    fn a_hit_lands_on_its_line_after_unsaved_lines_were_typed_above_it() {
        let bed = bed("hit-unsaved");
        bed.write("log.md", "# Log\none\ntwo\nneedle here\nlast\n");
        let mut app = bed.open("log.md");
        app.editor.set_cursor((1, 0));
        type_text(&mut app, "fresh");
        enter(&mut app);
        type_text(&mut app, "second");
        enter(&mut app);
        assert_eq!(app.editor.lines()[5], "needle here", "precondition: two lines typed above");
        assert!(search_and_open(&mut app, "needle"), "the search finds the line");
        assert_eq!(cursor_line(&app), "needle here", "the cursor is on the hit");
    }

    #[test]
    fn a_hit_lands_on_its_line_after_unsaved_lines_were_deleted_above_it() {
        let bed = bed("hit-unsaved-del");
        bed.write("log.md", "# Log\none\ntwo\nthree\nneedle here\nlast\n");
        let mut app = bed.open("log.md");
        // join "one" and "two" away: backspace over both lines
        app.editor.set_cursor((3, 0));
        for _ in 0..8 {
            press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        assert_eq!(app.editor.lines()[2], "needle here", "precondition: two lines deleted above");
        assert!(search_and_open(&mut app, "needle"), "the search finds the line");
        assert_eq!(cursor_line(&app), "needle here", "the cursor is on the hit");
    }

    #[test]
    fn a_hit_is_not_offered_for_a_line_the_open_note_has_since_changed() {
        let bed = bed("hit-stale");
        bed.write("log.md", "# Log\none\nneedle here\nlast\n");
        let mut app = bed.open("log.md");
        // "needle" becomes "noodle", and nothing has saved it yet
        app.editor.set_cursor((2, 6));
        for _ in 0..6 {
            press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        }
        type_text(&mut app, "noodle");
        assert_eq!(app.editor.lines()[2], "noodle here", "precondition: the line was retyped");
        // either the list has no such hit, or Enter lands on a line that has it
        if search_and_open(&mut app, "needle") {
            assert!(
                cursor_line(&app).contains("needle"),
                "a hit for \"needle\" opened on the line {:?}",
                cursor_line(&app)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_hit_lands_on_its_line_when_the_notes_folder_is_reached_through_a_symlink() {
        let bed = bed("hit-symlink");
        bed.write("log.md", "# Log\none\ntwo\nneedle here\nlast\n");
        let link = bed.root.join("notes");
        std::os::unix::fs::symlink(&bed.vault, &link).unwrap();
        fs::write(
            bed.home.join(".config/catcher/settings.md"),
            format!("- notes_dir: {}\n- window_title: off\n", link.display()),
        )
        .unwrap();
        let mut app = App::launch(Launch::Default).expect("the app launches");
        assert_eq!(
            fs::canonicalize(&app.active_note().path).unwrap(),
            bed.vault.join("log.md"),
            "precondition: the session opens on the one note"
        );
        app.editor.set_cursor((1, 0));
        type_text(&mut app, "fresh");
        enter(&mut app);
        type_text(&mut app, "second");
        enter(&mut app);
        assert_eq!(app.editor.lines()[5], "needle here", "precondition: two lines typed above");
        assert!(search_and_open(&mut app, "needle"), "the search finds the line");
        assert_eq!(cursor_line(&app), "needle here", "the cursor is on the hit");
    }
}

/// Enter on a hit whose note can no longer be opened.
mod search_hit_gone {
    use super::*;

    #[test]
    fn a_hit_that_cannot_be_opened_leaves_the_cursor_where_it_was() {
        let bed = bed("hit-gone");
        bed.write("here.md", "# Here\nfirst\nsecond\nthird\nfourth\nfifth\nsixth\nseventh\n");
        let gone = bed.write("sub/there.md", "# There\na\nb\nc\nd\nneedle\n");
        let mut app = bed.open("here.md");
        app.editor.set_cursor((1, 2));
        press(
            &mut app,
            KeyCode::Char('F'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        type_text(&mut app, "needle");
        let items = app.overlay_items();
        let at = items
            .iter()
            .position(|i| matches!(i, Item::Line(..)))
            .expect("precondition: the search finds the line in the other note");
        for _ in 0..at {
            press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        }
        // the file goes away between the search and the Enter
        fs::remove_file(&gone).unwrap();
        enter(&mut app);
        assert_eq!(
            app.active_note().path,
            bed.vault.join("here.md"),
            "precondition: the note on screen is still the one it was"
        );
        assert_eq!(app.editor.cursor, (1, 2), "the cursor moved in a note the hit is not in");
    }
}

/// `[[#Heading]]` links to a heading of the note it is in.
mod heading_link_not_tag {
    use super::*;

    const GUIDE: &str = "# Guide\nSee [[#Setup]] below, or [[#setup]].\n## Setup\nsteps\n";

    fn names(app: &App) -> Vec<String> {
        let (_, hits) = app.tag_filter.clone().expect("the tag list is open");
        let mut names: Vec<String> = hits.iter().map(|&i| app.open_index[i].name()).collect();
        names.sort();
        names
    }

    #[test]
    fn a_note_is_not_listed_under_a_tag_it_only_links_to_as_a_heading() {
        let bed = bed("tag-index");
        bed.write("guide.md", GUIDE);
        bed.write("real.md", "# Real\nthis one is #setup tagged\n");
        bed.write("both.md", "# Both\n[[#Setup]] and a real #setup\n## Setup\nx\n");
        let mut app = bed.open("real.md");
        // follow the real tag: ^O cut to the notes that carry it
        app.editor.set_cursor((1, 14));
        press(&mut app, KeyCode::Enter, KeyModifiers::ALT);
        assert_eq!(names(&app), vec!["both".to_string(), "real".to_string()]);
        assert!(crate::md::tags_in("See [[#Setup]] below").is_empty());
        assert_eq!(crate::md::tags_in("[[#Setup]] #setup").len(), 1);
    }

    #[test]
    fn following_a_heading_link_does_not_open_the_tag_list() {
        let bed = bed("tag-follow");
        bed.write("guide.md", GUIDE);
        bed.write("real.md", "# Real\nthis one is #setup tagged\n");
        let mut app = bed.open("guide.md");
        app.editor.set_cursor((1, 8));
        press(&mut app, KeyCode::Enter, KeyModifiers::ALT);
        assert!(app.tag_filter.is_none(), "alt+enter on [[#Setup]] opened the #Setup tag list");
        assert!(
            !matches!(
                crate::md::link_at("See [[#Setup]] below", 8),
                Some(crate::md::LinkTarget::Tag(_))
            ),
            "the cursor on [[#Setup]] is read as a tag"
        );
    }

    #[test]
    fn a_heading_link_is_not_drawn_the_way_a_tag_is() {
        let _bed = bed("tag-style");
        let tagged = crate::md::style_line("a real #setup tag");
        let tag = tagged
            .cells
            .iter()
            .find(|c| c.ch == 's')
            .expect("the tag is drawn")
            .style;
        let plain = tagged.cells.iter().find(|c| c.ch == 'a').unwrap().style;
        assert_ne!(tag, plain, "precondition: a tag is drawn in a style of its own");
        let line = crate::md::style_line("See [[#Setup]] below");
        let drawn: String = line.cells.iter().map(|c| c.ch).collect();
        assert!(drawn.contains("Setup"), "the link's text is drawn: {drawn:?}");
        for c in line.cells.iter().filter(|c| "#Setup".contains(c.ch)) {
            assert_ne!(c.style, tag, "{:?} of [[#Setup]] is drawn as a tag", c.ch);
        }
    }
}

/// CATCHER_DIR points one session at another folder.
mod env_dir_not_saved {
    use super::*;

    fn settings(bed: &Bed) -> String {
        fs::read_to_string(bed.home.join(".config/catcher/settings.md")).unwrap()
    }

    #[test]
    fn a_session_pointed_elsewhere_leaves_notes_dir_in_the_settings_file_alone() {
        let bed = bed("env-kept");
        let demo = bed.root.join("demo");
        fs::create_dir_all(&demo).unwrap();
        // the two-line settings file `bed` wrote: an older file, short of
        // most of today's settings
        std::env::set_var("CATCHER_DIR", &demo);
        let config = crate::config::Config::load().expect("the settings load");
        std::env::remove_var("CATCHER_DIR");
        assert_eq!(config.notes_dir, demo, "precondition: the session is pointed at CATCHER_DIR");
        let text = settings(&bed);
        assert!(
            !text.contains(demo.to_str().unwrap()),
            "the one-off folder was written into settings.md:\n{text}"
        );
        // and the next plain run is back in the vault
        let config = crate::config::Config::load().expect("the settings load");
        assert_eq!(config.notes_dir, bed.vault);
    }

    #[test]
    fn a_whole_session_under_the_environment_never_writes_it_to_the_settings_file() {
        let bed = bed("env-session");
        let demo = bed.root.join("demo");
        fs::create_dir_all(&demo).unwrap();
        fs::write(demo.join("scratch.md"), "# Scratch\nhello\n").unwrap();
        std::env::set_var("CATCHER_DIR", &demo);
        let mut app = App::launch(Launch::Default).expect("the app launches");
        assert_eq!(app.config.notes_dir, demo, "precondition: the session is pointed at CATCHER_DIR");
        type_text(&mut app, "typing");
        press(&mut app, KeyCode::Char('s'), KeyModifiers::CONTROL);
        shown(&mut app);
        std::env::remove_var("CATCHER_DIR");
        let text = settings(&bed);
        assert!(
            !text.contains(demo.to_str().unwrap()),
            "the one-off folder was written into settings.md:\n{text}"
        );
        let config = crate::config::Config::load().expect("the settings load");
        assert_eq!(config.notes_dir, bed.vault, "the next plain run is back in the vault");
    }

    #[test]
    fn a_settings_file_that_names_no_folder_does_not_gain_the_environments() {
        let bed = bed("env-none");
        let demo = bed.root.join("demo");
        fs::create_dir_all(&demo).unwrap();
        fs::write(
            bed.home.join(".config/catcher/settings.md"),
            "- theme: dark\n- window_title: off\n",
        )
        .unwrap();
        std::env::set_var("CATCHER_DIR", &demo);
        let config = crate::config::Config::load().expect("the settings load");
        std::env::remove_var("CATCHER_DIR");
        assert_eq!(config.notes_dir, demo, "precondition: the session is pointed at CATCHER_DIR");
        assert_eq!(
            config.attachments_dir,
            demo.join("attachments"),
            "the session's pictures still go under the folder it was pointed at"
        );
        let text = settings(&bed);
        assert!(
            !text.contains(demo.to_str().unwrap()),
            "the one-off folder was written into settings.md:\n{text}"
        );
    }
}

/// The reading view has the editor's folds.
mod reading_view_folds {
    use super::*;

    const PLAN: &str = "# Plan\nintro words\n## One\nalpha one\n- [ ] task one\n## Two\n- [ ] task two\nbeta two\n## Three\ngamma three\n";

    #[test]
    fn a_section_folded_in_the_editor_is_not_drawn_in_the_reading_view() {
        let bed = bed("read-hide");
        bed.write("plan.md", PLAN);
        let mut app = bed.open("plan.md");
        fold(&mut app, 2);
        read(&mut app);
        let page = screen(&mut app);
        for text in ["intro words", "One", "Two", "task two", "beta two", "Three", "gamma three"] {
            assert!(find_on(&page, text).is_some(), "{text:?} is on the page");
        }
        for text in ["alpha one", "task one"] {
            assert!(find_on(&page, text).is_none(), "{text:?} is under a fold and still drawn");
        }
        assert!(
            app.preview_checkboxes.iter().all(|(_, line)| *line != 4),
            "the checkbox under the fold is still there to click"
        );
    }

    /// The screen row that draws `needle`, as (char, style) cells.
    fn row_of(app: &mut App, needle: &str) -> Vec<(char, ratatui::style::Style)> {
        let mut term = Terminal::new(TestBackend::new(90, 40)).unwrap();
        term.draw(|f| crate::ui::draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let chars = |y: u16| -> Vec<char> {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect()
        };
        let want: Vec<char> = needle.chars().collect();
        let y = (0..buf.area.height)
            .find(|&y| chars(y).windows(want.len()).any(|w| w == want.as_slice()))
            .unwrap_or_else(|| panic!("{needle:?} is on the page"));
        (0..buf.area.width)
            .map(|x| {
                let cell = &buf[(x, y)];
                (cell.symbol().chars().next().unwrap_or(' '), cell.style())
            })
            .collect()
    }

    #[test]
    fn a_folded_heading_does_not_look_like_an_open_one() {
        let bed = bed("read-marked");
        bed.write("plan.md", PLAN);
        let mut app = bed.open("plan.md");
        read(&mut app);
        let open = row_of(&mut app, "Two");
        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(app.view == View::Edit, "precondition: ^P goes back to the editor");
        fold(&mut app, 5);
        read(&mut app);
        assert!(!drawn(&mut app, "beta two"), "the folded section is still drawn");
        let folded = row_of(&mut app, "Two");
        // a marker, a count, a colour: anything, so long as the page says
        // there is a section here that is not being shown
        assert!(
            folded != open,
            "nothing on the heading's row says its section is folded"
        );
    }

    #[test]
    fn everything_under_a_folded_heading_goes_with_it() {
        let bed = bed("read-blocks");
        bed.write(
            "plan.md",
            "# Plan\n## One\n```\nlet inside = 1;\n```\n| cella | cellb |\n| - | - |\n| cellc | celld |\n> quoted words\n### Deeper\nnested words\n## Two\nbeta two\n",
        );
        let mut app = bed.open("plan.md");
        fold(&mut app, 1);
        read(&mut app);
        let page = screen(&mut app);
        for text in ["let inside", "cella", "celld", "quoted words", "Deeper", "nested words"] {
            assert!(find_on(&page, text).is_none(), "{text:?} is under a fold and still drawn");
        }
        assert!(find_on(&page, "beta two").is_some(), "the next section is on the page");
    }

    #[test]
    fn a_click_on_a_heading_folds_it_and_another_unfolds_it() {
        let bed = bed("read-click");
        bed.write("plan.md", PLAN);
        let mut app = bed.open("plan.md");
        read(&mut app);
        let at = find_on(&screen(&mut app), "Two").expect("the heading is on the page");
        click(&mut app, at);
        assert!(app.folded_here(5), "a click on the heading folds its section");
        let page = screen(&mut app);
        assert!(find_on(&page, "beta two").is_none(), "the section is still drawn");
        assert!(find_on(&page, "task two").is_none(), "the section is still drawn");
        assert!(find_on(&page, "gamma three").is_some(), "the next section stays");
        assert!(find_on(&page, "alpha one").is_some(), "the section before stays");
        let at = find_on(&page, "Two").expect("the folded heading is on the page");
        click(&mut app, at);
        assert!(!app.folded_here(5), "a second click unfolds it");
        assert!(drawn(&mut app, "beta two"), "the section is back");
    }

    #[test]
    fn a_fold_made_in_the_reading_view_is_the_editors_fold_too() {
        let bed = bed("read-shared");
        bed.write("plan.md", PLAN);
        let mut app = bed.open("plan.md");
        read(&mut app);
        let at = find_on(&screen(&mut app), "Two").expect("the heading is on the page");
        click(&mut app, at);
        press(&mut app, KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert!(app.view == View::Edit, "precondition: ^P goes back to the editor");
        assert_eq!(folded(&app), vec![5]);
        let page = shown(&mut app);
        assert!(!page.iter().any(|l| l == "beta two"), "{page:?}");
        assert!(page.iter().any(|l| l == "gamma three"), "{page:?}");
    }

    #[test]
    fn a_checkbox_below_a_fold_still_ticks_its_own_line() {
        let bed = bed("read-checkbox");
        // front matter too: the reading view never draws it, and every line
        // still counts from the top of the file
        bed.write("plan.md", &format!("---\ntags: demo\n---\n{PLAN}"));
        let mut app = bed.open("plan.md");
        fold(&mut app, 5);
        read(&mut app);
        let page = screen(&mut app);
        assert!(find_on(&page, "task one").is_none(), "the folded section is still drawn");
        let (x, y) = find_on(&page, "task two").expect("the open section is on the page");
        // the box is drawn in front of the item's text
        let boxes: Vec<(u16, u16)> = app
            .preview_checkboxes
            .iter()
            .filter(|(r, _)| r.y == y)
            .map(|(r, _)| (r.x, r.y))
            .collect();
        assert_eq!(boxes.len(), 1, "one checkbox on the row of \"task two\" (text at column {x})");
        click(&mut app, boxes[0]);
        assert_eq!(app.editor.lines()[9], "- [x] task two", "the box that was clicked is ticked");
        assert_eq!(app.editor.lines()[7], "- [ ] task one", "and no other");
    }

    #[test]
    fn a_search_hit_inside_a_fold_is_on_the_page_in_the_reading_view() {
        let bed = bed("read-hit");
        bed.write("plan.md", PLAN);
        let mut app = bed.open("plan.md");
        fold(&mut app, 5);
        read(&mut app);
        assert!(!drawn(&mut app, "beta two"), "the folded section is still drawn");
        assert!(search_and_open(&mut app, "beta"), "precondition: the search finds the line");
        assert!(app.view == View::Preview, "precondition: the reading view is kept");
        assert!(drawn(&mut app, "beta two"), "the hit is not on the page");
    }
}
