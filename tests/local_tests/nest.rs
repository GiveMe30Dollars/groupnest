//! Test regarding nesting.
//!
//! `Nest` nodes affect logical newlines. Previously, `Nest` would only take affect on subsequent lines,
//! thus mandating linebreaks to be placed just after entering and exiting `Nest` scopes,
//! which is counterintuitive to an end-user.
//!
//! With the `LayoutEngine` now responsible for stripping extraneous padding (thus delaying their emission),
//! it can now identify when it is at a logical newline, and respond accordingly to `Nest` indentation and dedentation.

use expect_test::expect;
use groupnest::{
    BoxDoc,
    document::{Document, FlatFragment},
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
};

const SETTINGS : LayoutSettings = LayoutSettings {
    initial_mode: LayoutMode::Broken,
    min_width: 20,  // Default settings from here and below.
    max_width: 100,
    width_constraint: LayoutWidthConstraint::Relaxed,
};

#[test]
fn strip_trailing() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::from_text("a\n"),
        BoxDoc::nest(4, BoxDoc::nil())
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        a
    "#]]
    .assert_eq(&result);
    let lines = result.split('\n').collect::<Vec<_>>();
    assert_eq!(lines[1].len(), 0);
}

#[test]
fn strip_trailing_empty_string() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::from_text("a\n"),
        BoxDoc::nest(
            4,
            // Smart construction would canonize this, so we need to explicitly type it out.
            BoxDoc(Box::new(Document::Text(FlatFragment::new("").unwrap()))),
        ),
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        a
    "#]]
    .assert_eq(&result);
    let lines = result.split('\n').collect::<Vec<_>>();
    assert_eq!(lines[1].len(), 0);
}

#[test]
fn premature_enter() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::from_text("a\n"),
        BoxDoc::nest(4, BoxDoc::flat_text("b")),
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        a
            b"#]]
    .assert_eq(&result);
    let lines = result.split('\n').collect::<Vec<_>>();
    assert_eq!(lines[1].len(), 5);
}

#[test]
fn premature_exit() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::nest(4, BoxDoc::from_text("b\n")),
        BoxDoc::flat_text("c"),
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
            b
        c"#]]
    .assert_eq(&result);
    let lines = result.split('\n').collect::<Vec<_>>();
    assert_eq!(lines[1].len(), 1);
}

#[test]
/// The user-ergonomics test. This is how I expect most people to use this.
/// Combination of [`premature_enter`] and [`premature_exit`].
fn logical_newline() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::from_text("outer {\n"),
        BoxDoc::nest(4, BoxDoc::from_text("inner\n")),
        BoxDoc::flat_text("}"),
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        outer {
            inner
        }"#]]
    .assert_eq(&result);
    // Erroneous output:
    // ```
    // outer {
    // inner
    //     }
    // ```
}

#[test]
fn nest_around_multiline_break() {
    let doc: BoxDoc<()> = BoxDoc::nest(2,
        BoxDoc::sequence(vec![
            BoxDoc::flat_text("a "),
            BoxDoc::breaker("", "x\ny"),
            BoxDoc::flat_text(" b"),
            BoxDoc::breaker("", "\nif you\nchange your mind"),
        ]),
    );

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    // This needs to be its own thing, because `expect_test::expect` strips up to the least amount of common indentation.
    let expected = "  a x\n  y b\n  if you\n  change your mind";
    assert_eq!(&expected, &result);
}

#[test]
fn nest_in_current_line() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::flat_text("a"),
        BoxDoc::nest(2, BoxDoc::from_text("b\nc")),
    ]);

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        ab
          c"#]]
    .assert_eq(&result);
}

#[test]
fn nest_with_breaker_spaces() {
    let doc: BoxDoc<()> = BoxDoc::nest(2, 
        BoxDoc::breaker("", "a\n  b\n  c"));

    let result = doc.to_plaintext_with(SETTINGS).unwrap();
    let expected = "  a\n    b\n    c";
    assert_eq!(&expected, &result);
}
