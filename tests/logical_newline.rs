//! Test to assert that `Nest` nodes affect logical newlines.
//! 
//! Previously, `Nest` would only take affect on subsequent lines,
//! thus mandating linebreaks to be placed just after entering and exiting `Nest` scopes,
//! which is counterintuitive to an end-user.
//! 
//! With the `LayoutEngine` now responsible for stripping extraneous padding (thus delaying their emission),
//! it can now identify when it is at a logical newline, and respond accordingly to `Nest` indentation and dedentation.

use groupnest::{Arena, RefDocBuilder, GroupPolicy};
use expect_test::expect;

#[test]
fn logical_newline() {
    let arena = Arena::new();
    let builder: RefDocBuilder<'_, '_, ()> = RefDocBuilder::new(&arena);
    let doc = builder.group(builder.sequence(vec![
        builder.from_text("outer {\n"),
        builder.nest(4, builder.from_text("inner\n\n")),
        builder.flat_text("}")
    ]), GroupPolicy::ForceBreak);

    let result = doc.to_plaintext().unwrap();
    expect![[r#"
        outer {
            inner

        }"#]].assert_eq(&result);

    let lines = result.lines().collect::<Vec<_>>();
    // Assert that the third line contains no unecessary padding.
    assert_eq!(lines[2].len(), 0)
}