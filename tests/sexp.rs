//! The canonical S-expression test.

use expect_test::expect;
use groupnest::{
    Arena, Doc, DocBuilder, GroupPolicy,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
    renderer::PlaintextRenderer,
};

enum SExp {
    Atom(u32),
    List(Vec<SExp>),
}
impl SExp {
    fn to_doc<'a>(&'a self, builder: &mut DocBuilder<'static, 'a, ()>) -> Doc<'static, 'a, ()> {
        match self {
            SExp::Atom(num) => builder.flat_text(format!("{num:?}")),
            SExp::List(sexps) => {
                let child_separator = builder.breaker(" ", "\n");
                let children = sexps
                    .iter()
                    .map(|elem| elem.to_doc(builder))
                    .collect::<Vec<_>>();
                let inner = builder.sequence_intersperse_with(children, child_separator);

                let open_parens_separator = builder.breaker("", "\n");
                let close_parens_separator = builder.breaker("", "\n");
                let inner_with_newline = builder.sequence(vec![open_parens_separator, inner]);

                let nest = builder.nest(2, inner_with_newline);
                let open_parens = builder.flat_text("(");
                let close_parens = builder.flat_text(")");
                builder.grouped_sequence(
                    vec![open_parens, nest, close_parens_separator, close_parens],
                    GroupPolicy::Normal,
                )
            }
        }
    }
}

#[test]
fn generate_doc() {
    let template = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);
    let arena = Arena::new();
    let doc = template.to_doc(&mut DocBuilder::new(&arena));
    let layout = doc.as_layout();
    let result = PlaintextRenderer::render_to_string(layout).unwrap();
    expect!["((1) (2 3) (4 5 6))"].assert_eq(&result);
}

#[test]
fn cramped() {
    let template = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);
    let arena = Arena::new();
    let doc = template.to_doc(&mut DocBuilder::new(&arena));
    let layout = doc.as_layout_with(LayoutSettings {
        min_width: 0,
        max_width: 10,
        width_constraint: LayoutWidthConstraint::Relaxed,
        initial_mode: LayoutMode::Flat,
    });
    let result = PlaintextRenderer::render_to_string(layout).unwrap();
    expect![[r#"
        (
          (1)
          (2 3)
          (
            4
            5
            6
          )
        )"#]]
    .assert_eq(&result);
}
