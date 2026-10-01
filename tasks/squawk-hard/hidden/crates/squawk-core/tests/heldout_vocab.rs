//! What `RepoVocab::build` lists for a session's directory.

mod heldout_support;

use std::path::Path;

use heldout_support::{apply, context_for, git_repo, tempdir, widget, write, FILES};
use squawk_core::context::RepoVocab;

fn has(v: &RepoVocab, file: &str) -> bool {
    v.files.iter().any(|f| f == file)
}

#[test]
fn a_git_repo_lists_its_tracked_files_relative_to_the_cwd() {
    let (dir, ctx) = widget();
    let v = &ctx.vocab;
    assert_eq!(v.root, dir.path);
    for (file, _) in FILES {
        assert!(has(v, file), "{file} in {:?}", v.files);
    }
    assert!(
        v.files.iter().all(|f| !f.starts_with('/') && !f.starts_with("./")),
        "relative paths: {:?}",
        v.files
    );
    assert!(
        !v.files.iter().any(|f| f.starts_with(".git/")),
        "{:?}",
        v.files
    );
}

#[test]
fn ignored_build_output_is_not_listed() {
    let (dir, _) = widget();
    // .gitignore has target/.
    write(&dir.path, "target/debug/build/junk.rs", "");
    write(&dir.path, "target/debug/audio.rs", "");
    let ctx = context_for(&dir.path);
    assert!(has(&ctx.vocab, "src/audio.rs"));
    assert!(
        !ctx.vocab.files.iter().any(|f| f.starts_with("target/")),
        "{:?}",
        ctx.vocab.files
    );
    // So the copy under target/ does not make the real one ambiguous.
    assert_eq!(apply(&ctx, "open audio dot rs"), "open @src/audio.rs");
}

#[test]
fn file_names_are_listed_as_they_are_on_disk() {
    let dir = git_repo(&[
        ("docs/übersicht.md", ""),
        ("docs/日本語.md", ""),
        ("src/audio.rs", ""),
    ]);
    let ctx = context_for(&dir.path);
    for file in ["docs/übersicht.md", "docs/日本語.md", "src/audio.rs"] {
        assert!(has(&ctx.vocab, file), "{file} in {:?}", ctx.vocab.files);
    }
}

#[test]
fn a_session_in_a_subdirectory_gets_paths_relative_to_it() {
    let dir = git_repo(&[
        ("README.md", "# Mono\n"),
        ("app/src/main.rs", ""),
        ("app/src/audio.rs", ""),
        ("lib/src/lib.rs", ""),
    ]);
    let app = dir.path.join("app");
    let ctx = context_for(&app);
    assert_eq!(ctx.vocab.root, app);
    assert!(has(&ctx.vocab, "src/main.rs"), "{:?}", ctx.vocab.files);
    assert!(!has(&ctx.vocab, "app/src/main.rs"), "{:?}", ctx.vocab.files);
    assert_eq!(apply(&ctx, "open main dot rs"), "open @src/main.rs");
    assert_eq!(
        apply(&ctx, "then audio dot rs"),
        "then @src/audio.rs"
    );
}

#[test]
fn a_directory_outside_git_is_listed_too() {
    let dir = tempdir();
    write(&dir.path, "src/audio.rs", "");
    write(&dir.path, "notes.md", "");
    let ctx = context_for(&dir.path);
    assert!(has(&ctx.vocab, "src/audio.rs"), "{:?}", ctx.vocab.files);
    assert!(has(&ctx.vocab, "notes.md"), "{:?}", ctx.vocab.files);
    assert_eq!(apply(&ctx, "open audio dot rs"), "open @src/audio.rs");
}

#[test]
fn a_directory_that_is_gone_is_an_empty_vocab() {
    // Sanity: a real one is not empty.
    let (_dir, ctx) = widget();
    assert!(!ctx.vocab.files.is_empty());

    let gone = Path::new("/nonexistent/squawk/heldout");
    let v = RepoVocab::build(gone);
    assert!(v.files.is_empty(), "{:?}", v.files);
    assert_eq!(v.root, gone);
}
