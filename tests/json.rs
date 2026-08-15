//! A quick and dirty JSON formatter, which can display both inline and multiline;
//! a demonstration of a formatter of structured data.
//! The scope of this test does not include JSON parsing,
//! but does show how this crate can generate round-trip source text if parsing is implemented.
//!
//! Uses ArcDoc and ArcDocBuilder.

use expect_test::expect;
use groupnest::{
    ArcDoc, ArcDocBuilder,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
};
use indexmap::{IndexMap, indexmap};

struct Formatter {
    nest_amount: usize,
    has_trailing_comma: bool,
}

enum Json {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<Self>),
    Object(IndexMap<String, Self>),
}
impl Formatter {
    const STRICT: Self = Formatter {
        nest_amount: 2,
        has_trailing_comma: false,
    };
    const JSON5: Self = Formatter {
        nest_amount: 2,
        has_trailing_comma: true,
    };

    fn to_doc(&self, json: &Json, builder: &ArcDocBuilder<()>) -> ArcDoc<()> {
        match json {
            Json::Null => builder.flat_text("null"),
            Json::Boolean(value) => builder.flat_text(if *value { "true" } else { "false" }),
            Json::Number(value) => builder.flat_text(value.to_string()),
            Json::String(value) => builder.flat_text(format!("{value:?}")),

            Json::Array(jsons) => {
                let children_docs = jsons
                    .iter()
                    .map(|child| self.to_doc(child, builder))
                    .collect::<Vec<_>>();
                builder.grouped_sequence(vec![
                    builder.breaker("[", "[\n"),
                    builder.nest(
                        self.nest_amount,
                        builder
                            .sequence_intersperse_with(children_docs, builder.breaker(", ", ",\n")),
                    ),
                    builder.breaker(
                        "]",
                        if self.has_trailing_comma {
                            ",\n]"
                        } else {
                            "\n}"
                        },
                    ),
                ])
            }

            Json::Object(map) => {
                let children_entries = map
                    .iter()
                    .map(|(key, value)| {
                        builder.sequence(vec![
                            builder.flat_text(format!("{key:?}")),
                            builder.flat_text(": "),
                            self.to_doc(value, builder),
                        ])
                    })
                    .collect::<Vec<_>>();
                builder.grouped_sequence(vec![
                    builder.breaker("{", "{\n"),
                    builder.nest(
                        self.nest_amount,
                        builder.sequence_intersperse_with(
                            children_entries,
                            builder.breaker(", ", ",\n"),
                        ),
                    ),
                    builder.breaker(
                        "}",
                        if self.has_trailing_comma {
                            ",\n}"
                        } else {
                            "\n}"
                        },
                    ),
                ])
            }
        }
    }
}

const NARROW: LayoutSettings = LayoutSettings {
    initial_mode: LayoutMode::Broken,
    min_width: 0,
    max_width: 10,
    width_constraint: LayoutWidthConstraint::Relaxed,
};

#[test]
fn json_null() {
    let doc = Formatter::STRICT.to_doc(&Json::Null, &ArcDocBuilder::new());
    let result = doc.to_plaintext().unwrap();
    expect!["null"].assert_eq(&result);
}

#[test]
fn json_true() {
    let doc = Formatter::STRICT.to_doc(&Json::Boolean(true), &ArcDocBuilder::new());
    let result = doc.to_plaintext().unwrap();
    expect!["true"].assert_eq(&result);
}

#[test]
fn json_false() {
    let doc = Formatter::STRICT.to_doc(&Json::Boolean(false), &ArcDocBuilder::new());
    let result = doc.to_plaintext().unwrap();
    expect!["false"].assert_eq(&result);
}

#[test]
fn json_string() {
    let doc = Formatter::STRICT.to_doc(
        &Json::String(String::from("Lorem ipsum\ndolor sit amet")),
        &ArcDocBuilder::new(),
    );
    let result = doc.to_plaintext().unwrap();
    expect![[r#""Lorem ipsum\ndolor sit amet""#]].assert_eq(&result);
}

#[test]
fn json_array() {
    let doc = Formatter::STRICT.to_doc(
        &Json::Array((0..5).map(|i| Json::Number(i as f64)).collect()),
        &ArcDocBuilder::new(),
    );

    let inline = doc.to_plaintext().unwrap();
    expect!["[0, 1, 2, 3, 4]"].assert_eq(&inline);

    let multiline = doc.to_plaintext_with(NARROW).unwrap();
    expect![[r#"
        [
          0,
          1,
          2,
          3,
          4
        }"#]]
    .assert_eq(&multiline);
}

#[test]
fn json_object() {
    let doc = Formatter::STRICT.to_doc(
        &Json::Object(indexmap! {
            String::from("name") => Json::String(String::from("John")),
            String::from("age") => Json::Number(30.0),
            String::from("city") => Json::String(String::from("New York")),
        }),
        &ArcDocBuilder::new(),
    );

    let inline = doc.to_plaintext().unwrap();
    expect![[r#"{"name": "John", "age": 30, "city": "New York"}"#]].assert_eq(&inline);

    let multiline = doc.to_plaintext_with(NARROW).unwrap();
    expect![[r#"
        {
          "name": "John",
          "age": 30,
          "city": "New York"
        }"#]]
    .assert_eq(&multiline);
}

#[test]
fn json_mix() {
    // {"menu": {
    //   "id": "file",
    //   "value": "File",
    //   "popup": {
    //     "menuitem": [
    //       {"value": "New", "onclick": "CreateNewDoc()"},
    //       {"value": "Open", "onclick": "OpenDoc()"},
    //       {"value": "Close", "onclick": "CloseDoc()"}
    //     ]
    //   }
    // }}
    let json = Json::Object(indexmap! {
        String::from("menu") => Json::Object(indexmap! {
            String::from("id") => Json::String(String::from("file")),
            String::from("value") => Json::String(String::from("File")),
            String::from("popup") => Json::Object(indexmap! {
                String::from("menuitem") => Json::Array(vec![
                    Json::Object(indexmap! {
                        String::from("value") => Json::String(String::from("New")),
                        String::from("onclick") => Json::String(String::from("CreateNewDoc()")),
                    }),
                    Json::Object(indexmap! {
                        String::from("value") => Json::String(String::from("Open")),
                        String::from("onclick") => Json::String(String::from("OpenDoc()")),
                    }),
                    Json::Object(indexmap! {
                        String::from("value") => Json::String(String::from("Close")),
                        String::from("onclick") => Json::String(String::from("CloseDoc()")),
                    }),
                ])
            })
        })
    });
    const SETTINGS: LayoutSettings = LayoutSettings {
        min_width: 0,
        max_width: 60,
        width_constraint: LayoutWidthConstraint::Relaxed,
        initial_mode: LayoutMode::Flat,
    };

    let strict = Formatter::STRICT.to_doc(&json, &ArcDocBuilder::new());
    let strict_result = strict.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        {
          "menu": {
            "id": "file",
            "value": "File",
            "popup": {
              "menuitem": [
                {"value": "New", "onclick": "CreateNewDoc()"},
                {"value": "Open", "onclick": "OpenDoc()"},
                {"value": "Close", "onclick": "CloseDoc()"}
              }
            }
          }
        }"#]]
    .assert_eq(&strict_result);

    let json5 = Formatter::JSON5.to_doc(&json, &ArcDocBuilder::new());
    let json5_result = json5.to_plaintext_with(SETTINGS).unwrap();
    expect![[r#"
        {
          "menu": {
            "id": "file",
            "value": "File",
            "popup": {
              "menuitem": [
                {"value": "New", "onclick": "CreateNewDoc()"},
                {"value": "Open", "onclick": "OpenDoc()"},
                {"value": "Close", "onclick": "CloseDoc()"},
              ],
            },
          },
        }"#]]
    .assert_eq(&json5_result);
}
