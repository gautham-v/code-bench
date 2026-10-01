//! Chunks that are already an @mention, a URL, a path, an address or code
//! are never rewritten. Every case also holds something that is rewritten.

mod heldout_support;

use heldout_support::{apply, widget};

#[test]
fn an_existing_mention_is_left_alone() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "@src/audio.rs is fine but check main dot rs"),
        "@src/audio.rs is fine but check @src/main.rs"
    );
    assert_eq!(
        apply(&ctx, "not @docs/audio.rs, I mean audio dot rs"),
        "not @docs/audio.rs, I mean @src/audio.rs"
    );
}

#[test]
fn urls_and_addresses_are_left_alone() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "see https://example.com/audio.rs then main dot rs"),
        "see https://example.com/audio.rs then @src/main.rs"
    );
    assert_eq!(
        apply(&ctx, "mail you@example.com about main dot rs"),
        "mail you@example.com about @src/main.rs"
    );
}

#[test]
fn paths_outside_the_repo_are_left_alone() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "copy ~/notes/audio.rs over audio dot rs"),
        "copy ~/notes/audio.rs over @src/audio.rs"
    );
    assert_eq!(
        apply(&ctx, "copy /tmp/old/main.rs over main dot rs"),
        "copy /tmp/old/main.rs over @src/main.rs"
    );
}

#[test]
fn inline_code_is_left_alone() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "run `cargo test` on main dot rs"),
        "run `cargo test` on @src/main.rs"
    );
}

#[test]
fn repo_terms_do_not_reach_into_protected_chunks() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "the tidewell docs are at https://tidewell.dev/docs"),
        "the Tidewell docs are at https://tidewell.dev/docs"
    );
    assert_eq!(
        apply(&ctx, "call tidewell::run from the tidewell thread"),
        "call tidewell::run from the Tidewell thread"
    );
    assert_eq!(
        apply(&ctx, "@vendor/tidewell/lib.rs wraps tidewell"),
        "@vendor/tidewell/lib.rs wraps Tidewell"
    );
}
