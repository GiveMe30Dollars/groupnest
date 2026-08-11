//! Testing regarding nil nodes.
//!
//! # Notes on Construction
//!
//! Smart construction tends to eliminate `Nil` nodes where possible.
//! To be able to test these properly, manual construction tends to be required.

use groupnest::{
    BoxDoc, GroupPolicy, document::{Document, Sequence},
};
use expect_test::expect;

#[test]
fn nil_sequence() {
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Sequence(Sequence::new(
            vec![BoxDoc::nil()].into_boxed_slice(),
        ))
    ));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nil_sequence_multiple() {
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Sequence(Sequence::new(
            vec![BoxDoc::nil(); 5].into_boxed_slice(),
        ))
    ));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nil_group() {
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Group(GroupPolicy::Normal, BoxDoc::nil())
    ));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nil_nest() {
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Nest(100, BoxDoc::nil())
    ));

    let result = doc.to_plaintext().unwrap();
    assert!(result.is_empty())
}

#[test]
fn nil_leading() {
    const TEXT : &str = "Lorem ipsum dolor sit amet";
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Sequence(Sequence::new(
            vec![
                BoxDoc::nil(),
                BoxDoc::flat_text(TEXT),
            ].into_boxed_slice()
        ))
    ));

    let result = doc.to_plaintext().unwrap();
    expect!["Lorem ipsum dolor sit amet"].assert_eq(&result);
    assert_eq!(result.len(), TEXT.len());
}

#[test]
fn nil_trailing() {
    const TEXT : &str = "Lorem ipsum dolor sit amet";
    let doc: BoxDoc<()> = BoxDoc(Box::new(
        Document::Sequence(Sequence::new(
            vec![
                BoxDoc::flat_text(TEXT),
                BoxDoc::nil(),
            ].into_boxed_slice()
        ))
    ));

    let result = doc.to_plaintext().unwrap();
    expect!["Lorem ipsum dolor sit amet"].assert_eq(&result);
    assert_eq!(result.len(), TEXT.len());
}