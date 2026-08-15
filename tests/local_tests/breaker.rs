//! Test relating to the augmented Break node.

use expect_test::expect;
use groupnest::{BoxDoc, document::Break, layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint}};

const BREAK_FIRST : LayoutSettings = LayoutSettings {
    initial_mode: LayoutMode::Broken,
    min_width: 20,
    max_width: 100,
    width_constraint: LayoutWidthConstraint::Relaxed,
};

#[test]
fn flat_payload_invariant() {
    expect![[r#"
        FragmentError(
            ContainsLinebreak(
                "\r\n",
                4,
                "CRLF\r\n",
            ),
        )
    "#]].assert_debug_eq(&Break::new("CRLF\r\n", "\n").unwrap_err());
    expect![[r#"
        FragmentError(
            ContainsTab(
                ContainsTab(
                    4,
                    "tab!\t",
                ),
            ),
        )
    "#]].assert_debug_eq(&Break::new("tab!\t", "\n").unwrap_err());
}

#[test]
fn breaker_prefix() {
    let doc: BoxDoc<()> = BoxDoc::breaker("", ";\n");
    let result = doc.to_plaintext_with(BREAK_FIRST).unwrap();
    expect![[r#"
        ;
    "#]].assert_eq(&result);
}

#[test]
fn breaker_suffix() {
    let doc: BoxDoc<()> = BoxDoc::breaker("", "\n;");
    let result = doc.to_plaintext_with(BREAK_FIRST).unwrap();
    expect![[r#"

        ;"#]].assert_eq(&result);
}

#[test]
fn break_payload_invariant() {
    expect![[r#"
        BrokenPayloadHasNoNewline
    "#]].assert_debug_eq(&Break::new("", "no newline!").unwrap_err());
    expect![[r#"
        FragmentError(
            ContainsTab(
                ContainsTab(
                    5,
                    "tab!\n\t",
                ),
            ),
        )
    "#]].assert_debug_eq(&Break::new("", "tab!\n\t").unwrap_err());
}

/// Yes, this is intended behaviour.
#[test]
fn breaker_infix_indentation() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::flat_text("{"),
        BoxDoc::nest(2,
            BoxDoc::breaker("unseen", "\n// indented!\n")
        ),
        BoxDoc::flat_text("}")
    ]);

    let result = doc.to_plaintext_with(BREAK_FIRST).unwrap();
    expect![[r#"
        {
          // indented!
        }"#]].assert_eq(&result);
}