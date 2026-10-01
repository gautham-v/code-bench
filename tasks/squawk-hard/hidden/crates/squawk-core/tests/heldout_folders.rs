//! A spoken folder ("context slash mod dot rs") picks between files with
//! the same name, and has to be a folder the file is in.

mod heldout_support;

use heldout_support::{apply, widget};

#[test]
fn a_spoken_folder_picks_between_files_with_the_same_name() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "open context slash mod dot rs"),
        "open @src/context/mod.rs"
    );
    assert_eq!(
        apply(&ctx, "not store slash mod dot rs"),
        "not @src/store/mod.rs"
    );
    assert_eq!(
        apply(&ctx, "open src slash store slash mod dot rs"),
        "open @src/store/mod.rs"
    );
}

#[test]
fn a_folder_that_does_not_hold_the_file_leaves_all_of_it_alone() {
    let (_dir, ctx) = widget();
    // audio.rs is in src/, not in src/store/ or docs/.
    assert_eq!(
        apply(&ctx, "open store slash audio dot rs not main dot rs"),
        "open store slash audio dot rs not @src/main.rs"
    );
    assert_eq!(
        apply(&ctx, "open docs slash audio dot rs"),
        "open docs slash audio dot rs"
    );
    // src/ holds no mod.rs of its own, and under it there are two.
    assert_eq!(
        apply(&ctx, "open src slash mod dot rs"),
        "open src slash mod dot rs"
    );
    // The right folder still works.
    assert_eq!(
        apply(&ctx, "open src slash audio dot rs"),
        "open @src/audio.rs"
    );
}
