Yet another Wadler-style pretty printer, for fun and profit!

## Quick Start

> *Let’s pretty-print simple sexps!*
>
> We want to pretty print sexps like:
> ```ignore
> (1 2 3)
> ```
> or, if the line would be too long, like:
> ```ignore
> ((1)
>  (2 3)
>  (4 5 6))
> ```

A simple symbolic expression consists of a numeric atom or a nested ordered list of symbolic expression children.

```rust
pub enum SExp {
    Atom(u32),
    List(Vec<SExp>),
}
```

We define a conversion to a [`Document`] type. For simplicity we'll export an [`OwnedDoc`], though these patterns also work for builder-dependent Document types like [`RefDoc`].

```rust
use groupnest::{OwnedDoc, GroupPolicy};
# pub enum SExp {
#     Atom(u32),
#     List(Vec<SExp>),
# }

impl SExp {
    pub fn to_doc(&self) -> OwnedDoc<'_, ()> {
        match self {
            SExp::Atom(num) => OwnedDoc::flat_text(num.to_string()),
            SExp::List(children) => {
                let children_docs = children.iter()
                    .map(|child| child.to_doc())
                    .collect::<Vec<_>>();
                OwnedDoc::grouped_sequence(vec![
                    OwnedDoc::flat_text("("),
                    OwnedDoc::nest(1, 
                        OwnedDoc::sequence_intersperse_with(
                            children_docs,
                            OwnedDoc::breaker(" ", "\n"),
                        )
                    ),
                    OwnedDoc::flat_text(")"),
                ], GroupPolicy::Normal)
            }
        }
    }
}
```

Then, turning that into a `String` is as easy as a method call:

```rust
# use groupnest::{OwnedDoc, GroupPolicy};
# pub enum SExp {
#     Atom(u32),
#     List(Vec<SExp>),
# }
# 
# impl SExp {
#     pub fn to_doc(&self) -> OwnedDoc<'_, ()> {
#         match self {
#             SExp::Atom(num) => OwnedDoc::flat_text(num.to_string()),
#             SExp::List(children) => {
#                 let children_docs = children.iter()
#                     .map(|child| child.to_doc())
#                     .collect::<Vec<_>>();
#                 OwnedDoc::grouped_sequence(vec![
#                     OwnedDoc::flat_text("("),
#                     OwnedDoc::nest(1, 
#                         OwnedDoc::sequence_intersperse_with(
#                             children_docs,
#                             OwnedDoc::breaker(" ", "\n"),
#                         )
#                     ),
#                     OwnedDoc::flat_text(")"),
#                 ], GroupPolicy::Normal)
#             }
#         }
#     }
# }
# 
# fn main() {
    let example = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);

    let out : String = example.to_doc().to_plaintext().unwrap();
    
    use expect_test::expect;
    expect!["((1) (2 3) (4 5 6))"].assert_eq(&out);
# }
```

We can also test that nesting and grouping behaves as we expected, by configuring via `LayoutSettings`.

```rust
# use groupnest::{OwnedDoc, GroupPolicy};
# pub enum SExp {
#     Atom(u32),
#     List(Vec<SExp>),
# }
# 
# impl SExp {
#     pub fn to_doc(&self) -> OwnedDoc<'_, ()> {
#         match self {
#             SExp::Atom(num) => OwnedDoc::flat_text(num.to_string()),
#             SExp::List(children) => {
#                 let children_docs = children.iter()
#                     .map(|child| child.to_doc())
#                     .collect::<Vec<_>>();
#                 OwnedDoc::grouped_sequence(vec![
#                     OwnedDoc::flat_text("("),
#                     OwnedDoc::nest(1, 
#                         OwnedDoc::sequence_intersperse_with(
#                             children_docs,
#                             OwnedDoc::breaker(" ", "\n"),
#                         )
#                     ),
#                     OwnedDoc::flat_text(")"),
#                 ], GroupPolicy::Normal)
#             }
#         }
#     }
# }
# 
# fn main() {
#     let example = SExp::List(vec![
#         SExp::List(vec![SExp::Atom(1)]),
#         SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
#         SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
#     ]);
# 
    use groupnest::layout::LayoutSettings;
    let small_window_settings = LayoutSettings{
        min_width: 0,
        max_width: 10,
        ..Default::default()
    };

    let cramped : String = example.to_doc()
        .to_plaintext_with(small_window_settings)
        .unwrap();

    use expect_test::expect;
    expect![r#"
        ((1)
         (2 3)
         (4 5 6))"#].assert_eq(&cramped);
# }
```

## Why Does This Exist?

***One.*** I wanted this for internal use for my compiler project `hasklite` (a Haskell subset), and thought this would be a fun and quick project.

***Two.*** Existing crates, while mature in their own right, were unsuited for my usage or just had minor inconveniences that irked me.

In particular:  
1. Follows the original Wadler algebraic formulation, in which:
    - Concatenation is pairwise, leading to deeply-nested document trees.
    - Multiple concatenation options exist.
2. Extensions to Wadler (as is the case for most libraries) introduce redundancies in the node types available to the user.
    - Multiple linebreak node types, along with conditionally-displaying text (in addition to optional newlines).
3. Opaque, uninspectable document types.
4. Documentation for usage best described as Spartan, assuming that you have already read the Wadler papers, and then some.

In contrast, this implementation:  
1. Follows an alternate formulation by the Gleam language team, where:
    - Sequences are flattened boxed slices, reducing tree depth.
    - Concatenation is not primitive, but can be done via nested `Sequence` nodes.
      Callers are encouraged to collect their children before document construction.
2. Minimal node types.
    - Eight [`Document`] node types, with each having little to no semantic overlap.
        - `HardLinebreak`, which mandates a newline in the final output.
        - `Break`, augmented to support additional arbitrary text in its broken payload.
3. Pattern-matchable documents, with all contents inspectible and deconstructible. Invariants are maintained by construction of inspectible wrapper types.
4. Hopefully better documentation.


## Further Documentation

- For document construction, refer to the raw Wadler [`Document`] representation and its wrappers:
  - [`RefDoc`]: A document that takes immutable reference to its children and is arena-allocated.
    - Reduces heap fragmentation, *but:*
    - Increases verbosity in builder patterns due to the caller requiring to provide an arena and [`DocBuilder`] instance.
  - [`OwnedDoc`]: A document that owns its internals, in full, via `Box`.
    - Simpler to build and store, *but:*
    - May suffer from heap fragmentation and duplication of common nodes.

- For document consumption, refer to the [`Renderer`] trait and its implementors.
  - The input to [`Renderer`] implementors is formed via [`LayoutEngine`], which is accessed by convenience functions [`Document::as_layout`] and [`Document::as_layout_with`].
  - This crate natively supports plaintext rendering via [`PlaintextRenderer`] and convenience functions [`Document::to_plaintext`] and [`Document::to_plaintext_with`].
  - The optional feature flag `termcolor` enables [`termcolor`](https://docs.rs/termcolor/latest/termcolor/) support for annotations via [`ColorPatch`] and rendering via [`TermcolorRenderer`].

## Alternatives

There's quite a few! A non-exhaustive list of existing options:
- [`pretty`](https://crates.io/crates/pretty): The classic. Basically the original Wadler algorithm verbatim.
- [`prettyless`](https://crates.io/crates/prettyless): A fork of `pretty` with (allegedly) more ergonomic usage within Rust.
- [`pprint`](https://crates.io/crates/pprint): *In addition* to being a document constructor, also provides derivable pretty-printing for Rust datatypes. Basically a neater version of `Debug`, neat!
- [`sparkly`](https://crates.io/crates/sparkly): Built-in terminal and ANSI coloring support.

## Will This Be On [`crates.io`](https://crates.io/)?

Maybe, probably not.

[`Document`]: crate::document::Document
[`RefDoc`]: crate::RefDoc
[`OwnedDoc`]: crate::OwnedDoc
[`DocBuilder`]: crate::DocBuilder
[`Renderer`]: crate::renderer::Renderer
[`LayoutEngine`]: crate::layout::LayoutEngine
[`Document::as_layout`]: crate::document::Document::as_layout
[`Document::as_layout_with`]: crate::document::Document::as_layout_with
[`Document::to_plaintext`]: crate::document::Document::to_plaintext
[`Document::to_plaintext_with`]: crate::document::Document::to_plaintext_with
[`ColorPatch`]: crate::renderer::ColorPatch
[`TermcolorRenderer`]: crate::renderer::TermcolorRenderer