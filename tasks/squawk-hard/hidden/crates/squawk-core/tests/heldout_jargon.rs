//! The repo's own terms come out in the repo's spelling, and ordinary words
//! stay ordinary.

mod heldout_support;

use heldout_support::{apply, widget};

#[test]
fn terms_from_the_docs_and_the_package_name_are_collected() {
    let (_dir, ctx) = widget();
    let words = &ctx.vocab.words;
    for w in ["Tidewell", "Brightwave", "widget-core"] {
        assert!(words.iter().any(|x| x == w), "{w} in {words:?}");
    }
}

#[test]
fn a_term_gets_the_repos_casing() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "ask tidewell about it"),
        "ask Tidewell about it"
    );
    // From CLAUDE.md.
    assert_eq!(
        apply(&ctx, "send it to brightwave"),
        "send it to Brightwave"
    );
}

#[test]
fn an_acronym_gets_the_repos_casing() {
    let (_dir, ctx) = widget();
    let words = &ctx.vocab.words;
    assert!(words.iter().any(|x| x == "GPUI"), "GPUI in {words:?}");
    assert_eq!(apply(&ctx, "the gpui popover"), "the GPUI popover");
    assert_eq!(apply(&ctx, "(gpui)"), "(GPUI)");
}

#[test]
fn punctuation_around_a_term_is_kept() {
    let (_dir, ctx) = widget();
    assert_eq!(apply(&ctx, "(tidewell)"), "(Tidewell)");
    assert_eq!(
        apply(&ctx, "tidewell, brightwave, and the rest."),
        "Tidewell, Brightwave, and the rest."
    );
}

#[test]
fn a_term_already_right_is_not_changed() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "Tidewell runs tidewell and Brightwave"),
        "Tidewell runs Tidewell and Brightwave"
    );
}

#[test]
fn words_that_only_start_sentences_in_the_docs_stay_lowercase() {
    let (_dir, ctx) = widget();
    // The README has "The engine…", "It is fast.", "Open the app…", "Run
    // the tests…": capitals that start a sentence, not names.
    assert_eq!(
        apply(
            &ctx,
            "the engine is fast and it runs when you open the app over tidewell"
        ),
        "the engine is fast and it runs when you open the app over Tidewell"
    );
    assert_eq!(
        apply(&ctx, "run the tests and open tidewell"),
        "run the tests and open Tidewell"
    );
}

#[test]
fn terms_and_mentions_work_in_one_dictation() {
    let (_dir, ctx) = widget();
    assert_eq!(
        apply(&ctx, "the tidewell code in audio dot rs talks to brightwave"),
        "the Tidewell code in @src/audio.rs talks to Brightwave"
    );
}
