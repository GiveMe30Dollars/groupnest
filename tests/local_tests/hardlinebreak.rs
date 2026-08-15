//! Test relating to the hard linebreak.

use groupnest::BoxDoc;

#[test]
fn linebreak_once() {
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::flat_text("a"),
        BoxDoc::hard_linebreak(),
        BoxDoc::flat_text("b"),
    ]);
    let result = doc.to_plaintext().unwrap();
    assert_eq!(result.split('\n').count(), 2)
}

#[test]
fn linebreak_multiple() {
    const NUM: usize = 5;
    let doc: BoxDoc<()> = BoxDoc::sequence(vec![
        BoxDoc::flat_text("a"),
        BoxDoc::sequence(vec![BoxDoc::hard_linebreak(); NUM]),
        BoxDoc::flat_text("b"),
    ]);
    let result = doc.to_plaintext().unwrap();
    assert_eq!(result.split('\n').count(), NUM + 1)
}
