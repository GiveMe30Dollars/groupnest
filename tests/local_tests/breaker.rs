//! Test relating to the augmented Break node.

use expect_test::expect;
use groupnest::{BoxDoc, GroupPolicy};

/// Yes, this is intended behaviour.
#[test]
fn break_indentation() {
    let doc : BoxDoc<()> = BoxDoc::grouped_sequence_with(GroupPolicy::ForceBreak, vec![
        BoxDoc::flat_text("{"),
        BoxDoc::nest(2,
            BoxDoc::breaker("unseen", "\n# indented!\n")
        ),
        BoxDoc::flat_text("}")
    ]);

    let result = doc.to_plaintext().unwrap();
    expect![[r#"
        {
          # indented!
        }"#]].assert_eq(&result);
}