//! The canonical S-expression test.

use expect_test::expect;
use groupnest::{
    Arena, RefDoc, DocBuilder, GroupPolicy,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
    renderer::{PlaintextRenderer, RenderError},
};

enum SExp {
    Atom(u32),
    List(Vec<SExp>),
}
impl SExp {
    fn to_doc<'a>(&'a self, builder: &mut DocBuilder<'static, 'a, ()>) -> RefDoc<'static, 'a, ()> {
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
                    vec![
                        open_parens,
                        nest,
                        close_parens_separator,
                        close_parens
                    ],
                    GroupPolicy::Normal,
                )
            }
        }
    }

    fn to_string(&self) -> Result<String, RenderError> {
        let arena = Arena::new();
        let doc = self.to_doc(&mut DocBuilder::new(&arena));
        doc.to_plaintext()
    }
    fn to_string_with(&self, settings: LayoutSettings) -> Result<String, RenderError> {
        let arena = Arena::new();
        let doc = self.to_doc(&mut DocBuilder::new(&arena));
        doc.to_plaintext_with(settings)
    }
}

#[test]
fn generate_doc() {
    let sexp = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);
    let result = sexp.to_string().unwrap();
    expect!["((1) (2 3) (4 5 6))"].assert_eq(&result);
}

#[test]
fn cramped() {
    let sexp = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);
    const SETTINGS : LayoutSettings = LayoutSettings {
        min_width: 0,
        max_width: 10,
        width_constraint: LayoutWidthConstraint::Relaxed,
        initial_mode: LayoutMode::Flat,
    };
    let result = sexp.to_string_with(SETTINGS).unwrap();
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

#[test]
fn logical_newline() {
    let arena = Arena::new();
    let builder = DocBuilder::new(&arena);
    let sexp = builder.sequence(vec![
        builder.from_text("outer {\n"),
        builder.nest(4, builder.from_text("inner\n")),
        builder.flat_text("}")
    ]);
    let result = PlaintextRenderer::render_to_string(sexp.as_layout()).unwrap();
    expect![[r#"
        outer {
            inner
        }"#]].assert_eq(&result);
}
