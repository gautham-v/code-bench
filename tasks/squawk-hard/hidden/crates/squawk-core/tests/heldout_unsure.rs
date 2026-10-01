//! A reference that is not certain stays as spoken. Every case also holds a
//! reference that is certain, so "do nothing" does not pass.

mod heldout_support;

use heldout_support::{apply, context_for, tempdir, widget, write};

/// Same words and the same @mentions; the case of the other words is not
/// this test's business.
fn assert_same_words(got: &str, want: &str) {
    assert_eq!(got.to_lowercase(), want.to_lowercase(), "{got}");
    let mentions = |s: &str| -> Vec<String> {
        s.split_whitespace()
            .filter(|w| w.contains('@'))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(mentions(got), mentions(want), "{got}");
}

#[test]
fn two_files_with_the_same_name_are_not_guessed_between() {
    let (_dir, ctx) = widget();
    // src/context/mod.rs and src/store/mod.rs.
    assert_eq!(
        apply(&ctx, "open mod dot rs and audio dot rs"),
        "open mod dot rs and @src/audio.rs"
    );
}

#[test]
fn a_file_the_repo_does_not_have_is_left_alone() {
    let (_dir, ctx) = widget();
    assert_same_words(
        &apply(&ctx, "open widget dot rs and audio dot rs"),
        "open widget dot rs and @src/audio.rs",
    );
    assert_eq!(
        apply(&ctx, "rename audio dot py to audio dot rs"),
        "rename audio dot py to @src/audio.rs"
    );
}

#[test]
fn ordinary_words_that_happen_to_be_file_stems_stay_words() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "the audio is choppy in audio dot rs"),
        "the audio is choppy in @src/audio.rs"
    );
    assert_eq!(
        apply(&ctx, "use the main branch for main dot rs"),
        "use the main branch for @src/main.rs"
    );
    assert_same_words(
        &apply(&ctx, "cargo is slow, check cargo toml"),
        "cargo is slow, check @Cargo.toml",
    );
    assert_same_words(
        &apply(&ctx, "claude should know about audio dot rs"),
        "claude should know about @src/audio.rs",
    );
    assert_eq!(
        apply(&ctx, "the store and the context both read audio dot rs"),
        "the store and the context both read @src/audio.rs"
    );
}

#[test]
fn an_extension_that_is_also_a_word_needs_its_dot() {
    let (_dir, ctx) = widget();
    // cmd/run.go is there; "run go test" is still just English.
    assert_eq!(
        apply(&ctx, "then run go test again and open run dot go"),
        "then run go test again and open @cmd/run.go"
    );
}

#[test]
fn a_spoken_dot_that_names_no_file_stays() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "version two dot five of audio dot rs"),
        "version two dot five of @src/audio.rs"
    );
}

#[test]
fn an_empty_directory_offers_nothing() {
    let (_dir, ctx) = widget();
    assert_eq!(apply(&ctx, "open audio dot rs"), "open @src/audio.rs");

    let empty = tempdir();
    let nothing = context_for(&empty.path);
    assert_eq!(apply(&nothing, "open audio dot rs"), "open audio dot rs");

    // Same words, another repo: the answer comes from that repo.
    let other = tempdir();
    write(&other.path, "lib/audio.rs", "");
    let ctx = context_for(&other.path);
    assert_eq!(apply(&ctx, "open audio dot rs"), "open @lib/audio.rs");
}
