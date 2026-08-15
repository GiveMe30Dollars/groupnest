//! Tests regarding annotations.

use expect_test::expect;
use groupnest::BoxDoc;

/// Annotations apply to anything within the inner document, even Nil.
#[test]
fn annotation_nil() {
    let doc: BoxDoc<_> = BoxDoc::annotation(String::from("bold!"), BoxDoc::nil());
    let events = doc.as_layout().collect::<Vec<_>>();

    assert!(!events.is_empty());
    expect![[r#"
        [
            PushAnnotation(
                "bold!",
            ),
            PopAnnotation,
        ]
    "#]]
    .assert_debug_eq(&events);
}
