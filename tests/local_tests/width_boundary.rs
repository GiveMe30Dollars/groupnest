//! Tests regarding the width determination of nodes.

use expect_test::expect;
use groupnest::{
    BoxDoc, GroupPolicy,
    document::FlatFragment,
    layout::{LayoutSettings, LayoutWidthConstraint},
};

#[test]
fn exact_fit() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(
        vec![
            BoxDoc::flat_text("abc"),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("def"),
        ],
        GroupPolicy::Normal,
    );
    let settings = LayoutSettings {
        min_width: 0,
        max_width: 7,
        ..Default::default()
    };
    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abc def"].assert_eq(&result);
}

#[test]
fn off_by_one() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(
        vec![
            BoxDoc::flat_text("abc"),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("def"),
        ],
        GroupPolicy::Normal,
    );
    let settings = LayoutSettings {
        min_width: 0,
        max_width: 6, // <- !!!
        ..Default::default()
    };
    let result = doc.to_plaintext_with(settings).unwrap();
    expect![[r#"
        abc
        def"#]]
    .assert_eq(&result);
}

#[test]
fn width_zero() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(
        vec![
            BoxDoc::flat_text("abc"),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("def"),
        ],
        GroupPolicy::Normal,
    );
    let settings = LayoutSettings {
        min_width: 0,
        max_width: 0, // <- !!!
        ..Default::default()
    };
    let result = doc.to_plaintext_with(settings).unwrap();
    expect![[r#"
        abc
        def"#]]
    .assert_eq(&result);
}

#[test]
fn width_zero_strict() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(
        vec![
            BoxDoc::flat_text("abc"),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("def"),
        ],
        GroupPolicy::Normal,
    );
    let settings = LayoutSettings {
        min_width: 0,
        max_width: 0, // <- !!!
        width_constraint: LayoutWidthConstraint::Strict,
        ..Default::default()
    };
    let result = doc.to_plaintext_with(settings).unwrap_err();
    expect![[r#"
        LayoutError(
            WidthExceeded {
                line_num: 1,
                max_width: 0,
                line_width: 3,
            },
        )
    "#]]
    .assert_debug_eq(&result);
}

#[test]
fn nesting_fit() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(
        vec![
            BoxDoc::flat_text("["),
            BoxDoc::nest(
                2,
                BoxDoc::sequence(vec![BoxDoc::breaker(" ", "\n"), BoxDoc::flat_text("x")]),
            ),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("]"),
        ],
        GroupPolicy::Normal,
    );
    let width_of = |max_width| LayoutSettings {
        min_width: 0,
        max_width,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(width_of(5)).unwrap();
    expect!["[ x ]"].assert_eq(&result);

    let result = doc.to_plaintext_with(width_of(4)).unwrap();
    expect![[r#"
        [
          x
        ]"#]]
    .assert_eq(&result);
}

#[test]
fn long_atom_never_splits() {
    let doc: BoxDoc<()> = BoxDoc::flat_text("abcdefghijklmnop");
    let settings = LayoutSettings {
        min_width: 0,
        max_width: 5,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abcdefghijklmnop"].assert_eq(&result);
}

#[test]
fn unicode_width() {
    let width_of = |payload| FlatFragment::new(payload).unwrap().unicode_width();
    assert_eq!(width_of("Hello World!"), 12);
    assert_eq!(width_of("αβγ"), 3);
    // CJK characters have a Unicode character width of 2.
    assert_eq!(width_of("你好世界"), 8);
}
