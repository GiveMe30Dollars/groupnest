use groupnest::{Arena, DocBuilder, GroupPolicy};
use expect_test::expect;

#[test]
fn logical_newline() {
    let arena = Arena::new();
    let builder: DocBuilder<'_, '_, ()> = DocBuilder::new(&arena);
    let doc = builder.group(builder.sequence(vec![
        builder.from_text("outer {\n"),
        builder.nest(4, builder.from_text("inner\n")),
        builder.flat_text("}")
    ]), GroupPolicy::ForceBreak);

    let result = doc.to_plaintext().unwrap();
    expect![[r#"
        outer {
            inner
        }"#]].assert_eq(&result);
}