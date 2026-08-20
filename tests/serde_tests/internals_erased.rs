use expect_test::expect;
use proptest::prelude::*;
use serde_json::{from_str, to_string, to_string_pretty};

use groupnest::{ArcDoc, ArcDocBuilder, BoxDoc, document::FlatFragment};

use crate::fuzzing::{arbitrary_boxdoc_leaf, arbitrary_settings};

#[test]
fn flat_fragment_erased() {
    const PAYLOAD: &str = "lorem ipsum dolor sit amet";
    let fragment = FlatFragment::new(PAYLOAD).unwrap();

    let serialized = to_string(&fragment).unwrap();
    expect![[r#""lorem ipsum dolor sit amet""#]].assert_eq(&serialized);

    let restored: FlatFragment<'_> = from_str(&serialized).unwrap();
    assert_eq!(fragment, restored);
}
proptest! {
    #[test]
    fn flat_fragment_erasure_stress(payload in "[a-zA-Z0-9]{0, 100}") {
        let fragment = FlatFragment::new(payload.clone()).unwrap();
        let serialized = to_string(&fragment).unwrap();
        assert_eq!(serialized, format!("\"{payload}\""));
        let restored : FlatFragment<'_> = from_str(&serialized).unwrap();
        assert_eq!(fragment, restored);
    }
}

#[test]
fn sequence_erased() {
    let builder: ArcDocBuilder<()> = ArcDocBuilder::new();
    let document: ArcDoc<()> = builder.sequence(
        (0..5)
            .into_iter()
            .map(|i| builder.flat_text(i.to_string()))
            .collect(),
    );

    let serialized = to_string_pretty(&document).unwrap();
    // Expecting: no `layout_mode_observable` or `break_status` fields.
    expect![[r#"
        {
          "Sequence": [
            {
              "Text": "0"
            },
            {
              "Text": "1"
            },
            {
              "Text": "2"
            },
            {
              "Text": "3"
            },
            {
              "Text": "4"
            }
          ]
        }"#]]
    .assert_eq(&serialized);

    let restored: ArcDoc<()> = from_str(&serialized).unwrap();
    assert_eq!(document, restored);
}
// doc_serde has indirectly stress-tested this one, but to be thorough:
macro_rules! payload_to_comparable {
    ($document:expr, $settings:expr) => {
        ($document)
            .to_plaintext_with($settings)
            .map_err(|error: groupnest::renderer::RenderError| error.to_string())
    };
}
proptest! {
    #[test]
    fn sequence_erasure_stress(document in prop::collection::vec(arbitrary_boxdoc_leaf(), 0..10).prop_map(BoxDoc::sequence), settings in arbitrary_settings()) {
        let serialized = to_string_pretty(&document).unwrap();
        let restored : BoxDoc<()> = from_str(&serialized).unwrap();
        assert_eq!(
            payload_to_comparable!(document, settings.clone()),
            payload_to_comparable!(restored, settings),
        )
    }
}

#[test]
fn bad_flat_fragment() {
    let contains_newline = r#""contains\nnewline!""#;
    let error = from_str::<FlatFragment<'_>>(contains_newline).expect_err("Should be error!");
    expect![[
        r#"Contains linebreak sequence "\n" at byte offset 8 of string '"contains\nnewline!"'."#
    ]]
    .assert_eq(&error.to_string());
}
