//! The canonical S-expression test, originating from `pretty` that every subsequent Wadler pretty printer library copies.
//! 
//! If it ain't broke, why change it?
//! 
//! > *Let’s pretty-print simple sexps!*
//! >
//! > We want to pretty print sexps like:
//! > ```ignore
//! > (1 2 3)
//! > ```
//! > or, if the line would be too long, like:
//! > ```ignore
//! > ((1)
//! >  (2 3)
//! >  (4 5 6))
//! > ```

use expect_test::expect;
use groupnest::{
    Arena, RefDoc, RefDocBuilder, GroupPolicy,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
    renderer::RenderError,
};

enum SExp {
    Atom(u32),
    List(Vec<SExp>),
}
impl SExp {
    fn to_doc<'a>(&'a self, builder: &RefDocBuilder<'static, 'a, ()>) -> RefDoc<'static, 'a, ()> {
        match self {
            SExp::Atom(num) => builder.flat_text(format!("{num:?}")),
            SExp::List(children) => {
                let children_docs = children.iter()
                    .map(|child| child.to_doc(builder))
                    .collect::<Vec<_>>();
                builder.grouped_sequence(vec![
                    builder.flat_text("("),
                    builder.nest(1,
                        builder.sequence_intersperse_with(
                            children_docs,
                            builder.breaker(" ", "\n"),
                        )
                    ),
                    builder.flat_text(")"),
                ], GroupPolicy::Normal)
            }
        }
    }

    fn to_string(&self) -> Result<String, RenderError> {
        let arena = Arena::new();
        let doc = self.to_doc(&RefDocBuilder::new(&arena));
        doc.to_plaintext()
    }
    fn to_string_with(&self, settings: LayoutSettings) -> Result<String, RenderError> {
        let arena = Arena::new();
        let doc = self.to_doc(&RefDocBuilder::new(&arena));
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
        ((1)
         (2 3)
         (4 5 6))"#]]
    .assert_eq(&result);
}