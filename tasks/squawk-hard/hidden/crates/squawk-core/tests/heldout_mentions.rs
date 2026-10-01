//! Spoken file names become @mentions relative to the session's cwd.

mod heldout_support;

use heldout_support::{apply, widget};
use squawk_core::cleanup::CleanupOptions;
use squawk_core::dictionary::Dictionary;
use squawk_core::pipeline;

#[test]
fn a_name_spoken_with_dot_resolves_to_its_path() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "look at audio dot rs"),
        "look at @src/audio.rs"
    );
    assert_eq!(apply(&ctx, "open main dot rs"), "open @src/main.rs");
}

#[test]
fn an_extension_said_without_dot_resolves() {
    let (_dir, ctx) = widget();
    assert_eq!(apply(&ctx, "open cargo toml"), "open @Cargo.toml");
}

#[test]
fn the_claude_md_resolves() {
    let (_dir, ctx) = widget();
    let got = apply(&ctx, "read the claude md first");
    // The article may go with the name or stay.
    assert!(
        got == "read @CLAUDE.md first" || got == "read the @CLAUDE.md first",
        "{got}"
    );
}

#[test]
fn a_name_of_several_words_resolves() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "take a look at status item dot rs"),
        "take a look at @src/status_item.rs"
    );
}

#[test]
fn the_words_match_whatever_way_the_file_name_joins_them() {
    let (_dir, ctx) = widget();
    // web/ContentView.tsx and scripts/build-release.sh.
    assert_eq!(
        apply(&ctx, "open content view dot tsx"),
        "open @web/ContentView.tsx"
    );
    assert_eq!(
        apply(&ctx, "then run build release dot sh"),
        "then run @scripts/build-release.sh"
    );
}

#[test]
fn a_file_deep_in_the_tree_gets_its_whole_path() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "popover dot rs needs work"),
        "@crates/app/src/ui/popover.rs needs work"
    );
}

#[test]
fn several_references_in_one_dictation() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "compare audio dot rs with main dot rs"),
        "compare @src/audio.rs with @src/main.rs"
    );
}

#[test]
fn punctuation_after_a_reference_is_kept() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "fix audio dot rs, then run it"),
        "fix @src/audio.rs, then run it"
    );
    assert_eq!(apply(&ctx, "is it audio dot rs?"), "is it @src/audio.rs?");
}

#[test]
fn a_capitalised_first_word_still_resolves() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "Audio dot rs is broken"),
        "@src/audio.rs is broken"
    );
}

#[test]
fn text_without_references_comes_back_as_it_was() {
    let (_dir, ctx) = widget();
    // Sanity for the cases above: the repo is seen at all.
    assert_eq!(apply(&ctx, "open main dot rs"), "open @src/main.rs");
    assert_eq!(apply(&ctx, ""), "");
    assert_eq!(
        apply(&ctx, "ship it on Friday, okay?"),
        "ship it on Friday, okay?"
    );
}

#[test]
fn through_the_whole_pipeline() {
    let (_dir, ctx) = widget();
    let opts = CleanupOptions::default();
    let dict = Dictionary::default();
    let got = pipeline::finish("Um, look at audio dot rs, okay?", &dict, Some(&ctx), &opts);
    assert_eq!(got, "Look at @src/audio.rs, okay?");
    // A sentence-ending period after a path may stay or go.
    let got = pipeline::finish("Um, open cargo toml.", &dict, Some(&ctx), &opts);
    assert!(got == "Open @Cargo.toml" || got == "Open @Cargo.toml.", "{got}");
    // A mention that opens the dictation is not capitalised.
    let got = pipeline::finish("audio dot rs is broken again", &dict, Some(&ctx), &opts);
    assert_eq!(got, "@src/audio.rs is broken again.");
    // Without a session nothing is a mention.
    let got = pipeline::finish("Um, look at audio dot rs, okay?", &dict, None, &opts);
    assert_eq!(got, "Look at audio dot rs, okay?");
}

#[test]
fn the_dictionary_still_applies_around_a_mention() {
    let (_dir, ctx) = widget();
    let opts = CleanupOptions::default();
    let dict = Dictionary::parse("cloud code -> Claude Code\n");
    let got = pipeline::finish(
        "ask cloud code to fix audio dot rs today",
        &dict,
        Some(&ctx),
        &opts,
    );
    assert_eq!(got, "Ask Claude Code to fix @src/audio.rs today.");
}
