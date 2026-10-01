//! Names with more than one dot, or a leading one, are taken whole: a
//! reference is the whole name that was said, never a piece of it.

mod heldout_support;

use heldout_support::{apply, widget};

#[test]
fn a_name_with_more_than_one_dot_resolves_whole() {
    let (_dir, ctx) = widget();
    // config.json and config.json.example are both there.
    assert_eq!(
        apply(&ctx, "copy config dot json dot example to config dot json"),
        "copy @config.json.example to @config.json"
    );
    assert_eq!(
        apply(&ctx, "open vite dot config dot ts"),
        "open @web/vite.config.ts"
    );
}

#[test]
fn a_name_that_starts_with_a_dot_resolves() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "add target to dot gitignore"),
        "add target to @.gitignore"
    );
}

#[test]
fn a_longer_name_the_repo_does_not_have_is_not_cut_short() {
    let (_dir, ctx) = widget();
    // There is no audio.rs.orig: mentioning audio.rs would name another file.
    assert_eq!(
        apply(&ctx, "delete audio dot rs dot orig but keep audio dot rs"),
        "delete audio dot rs dot orig but keep @src/audio.rs"
    );
    assert_eq!(
        apply(&ctx, "diff config dot json dot bak"),
        "diff config dot json dot bak"
    );
}
