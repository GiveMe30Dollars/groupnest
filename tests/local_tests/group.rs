//! Tests for grouping, sequences, and group policies for layout decisions.

use expect_test::expect;
use groupnest::{
    BoxDoc, GroupPolicy, document::{Document, Sequence}, layout::LayoutSettings,
};

#[test]
fn empty_group() {
    // Construction by hand.
    let doc: BoxDoc<()> = BoxDoc(Box::new(Document::Group(
        GroupPolicy::Normal,
        BoxDoc(Box::new(Document::Sequence(Sequence::new(
            Vec::new().into_boxed_slice(),
        )))),
    )));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nil_group() {
    // Construction by hand.
    let doc: BoxDoc<()> = BoxDoc(Box::new(Document::Group(
        GroupPolicy::Normal,
        BoxDoc::nil(),
    )));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nested_groups() {
    let doc: BoxDoc<()> = BoxDoc::group(
        BoxDoc::grouped_sequence(vec![
            BoxDoc::flat_text("a"),
            BoxDoc::breaker(" ", "\n"),
            BoxDoc::flat_text("b"),
        ], GroupPolicy::Normal),
    GroupPolicy::Normal);

    let width_of = |max_width| LayoutSettings {
        min_width: 0,
        max_width,
        ..Default::default()
    };

    let result_flat = doc.to_plaintext_with(width_of(3)).unwrap();
    expect!["a b"].assert_eq(&result_flat);

    let result_broken = doc.to_plaintext_with(width_of(2)).unwrap();
    expect![[r#"
        a
        b"#]].assert_eq(&result_broken);
}

#[test]

fn force_break() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::breaker(" ", "\n"),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::ForceBreak);

    let result = doc.to_plaintext().unwrap();
    expect![[r#"
        abc
        def"#]].assert_eq(&result);
}

#[test]

fn flat_if_possible() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::breaker(" ", "\n"),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 3,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abc def"].assert_eq(&result);
}

#[test]
fn flat_against_hardline() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::hard_linebreak(),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 100,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect![[r#"
        abc
        def"#]].assert_eq(&result);
}

#[test]
fn flat_against_forced_breaker() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::group(BoxDoc::breaker(" ", "\n"), GroupPolicy::ForceBreak),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 100,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect![[r#"
        abc

        def"#]].assert_eq(&result);
}

#[test]
fn flat_overrides_inner_normal() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::group(BoxDoc::breaker(" ", "\n"), GroupPolicy::Normal),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 3,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abc def"].assert_eq(&result);
}

#[test]
fn flat_against_unobservable_group() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::group(BoxDoc::flat_text(" "), GroupPolicy::ForceBreak),
        BoxDoc::flat_text("def"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 3,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abc def"].assert_eq(&result);
}

/// The breaker should not be forced broken, and thus wll be displayed as flat.
#[test]
fn flat_against_normal_breaker_in_double_groups() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::group(
            BoxDoc::group(BoxDoc::breaker(" ", "\n"), GroupPolicy::Normal),
            GroupPolicy::ForceBreak),
        BoxDoc::flat_text("def"),
        BoxDoc::breaker(".", "\n"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 3,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect!["abc def."].assert_eq(&result);
}

/// The breaker should be forced broken, and thus wll be displayed as flat.
#[test]
fn flat_against_forced_breaker_in_double_groups() {
    let doc: BoxDoc<()> = BoxDoc::grouped_sequence(vec![
        BoxDoc::flat_text("abc"),
        BoxDoc::group(
            BoxDoc::group(BoxDoc::breaker(" ", "\n"), GroupPolicy::ForceBreak),
            GroupPolicy::Normal),
        BoxDoc::flat_text("def"),
        BoxDoc::breaker(".", "\n"),
    ], GroupPolicy::FlatIfPossible);

    let settings = LayoutSettings {
        min_width: 0,
        max_width: 3,
        ..Default::default()
    };

    let result = doc.to_plaintext_with(settings).unwrap();
    expect![[r#"
        abc
        def
    "#]].assert_eq(&result);
}