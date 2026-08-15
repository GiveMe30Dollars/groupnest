# groupnest

Yet another Wadler-style pretty printer, for fun and profit!

### Quick Start

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

We define a conversion to a [`Document`] type. For simplicity we'll export an [`BoxDoc`], though these patterns also work for builder-dependent [`Document`] types like [`RefDoc`] and [`ArcDoc`].

```rust
use groupnest::BoxDoc;

impl SExp {
    pub fn to_doc(&self) -> BoxDoc<()> {
        match self {
            SExp::Atom(num) => BoxDoc::flat_text(num.to_string()),
            SExp::List(children) => {
                let children_docs = children.iter()
                    .map(|child| child.to_doc())
                    .collect::<Vec<_>>();
                BoxDoc::grouped_sequence(vec![
                    BoxDoc::flat_text("("),
                    BoxDoc::nest(1, 
                        BoxDoc::sequence_intersperse_with(
                            children_docs,
                            BoxDoc::breaker(" ", "\n"),
                        )
                    ),
                    BoxDoc::flat_text(")"),
                ])
            }
        }
    }
}
```

Then, turning that into a [`String`] is as easy as a method call:

```rust
    let example = SExp::List(vec![
        SExp::List(vec![SExp::Atom(1)]),
        SExp::List(vec![SExp::Atom(2), SExp::Atom(3)]),
        SExp::List(vec![SExp::Atom(4), SExp::Atom(5), SExp::Atom(6)]),
    ]);

    let out : String = example.to_doc().to_plaintext().unwrap();
    
    use expect_test::expect;
    expect!["((1) (2 3) (4 5 6))"].assert_eq(&out);
```

We can also test that nesting and grouping behaves as we expected, by configuring via [`LayoutSettings`].

```rust
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
```

### Why Does This Exist?

***One.*** I wanted this for internal use for my compiler project `hasklite` (a Haskell subset), and thought this would be a fun and quick project.

***Two.*** Existing crates, while mature in their own right, were unsuited for my usage or just had minor inconveniences that irked me.

*In particular:*

1. Most libraries follow the original Wadler algebraic formulation, in which:
    - Concatenation is pairwise, leading to deeply-nested document trees.
    - Multiple concatenation options exist, especially for document algebra which extends upon Wadler.
2. Extensions to Wadler (as is the case for most libraries) introduce redundancies in the node types available to the user.
    - Multiple linebreak node types, along with conditionally-displaying text (in addition to optional newlines).
3. Opaque, uninspectable document types.
4. Documentation for usage best described as Spartan, assuming that you have already read the Wadler papers, and then some.

*In contrast, this implementation:*

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


### Further Documentation

- For document construction, refer to the raw Wadler [`Document`] representation and its wrappers implementing [`Deref<Target = Document<...>>`]:
  - [`RefDoc`]: A document that takes immutable reference to its children and is arena-allocated.
    - Reduces heap fragmentation, *but:*
    - Increased complexity in builder patterns and usage due to requiring a [`RefDocBuilder`] and backing [`Arena`].
    - Prefer this for transient documents, i.e. those that are rendered in the same scope as its allocating arena.
  - [`BoxDoc`]: A document that owns its internals, in full, via [`Box`].
    - Simple to build statically, store and send between threads, *but:*
    - May suffer from heap fragmentation and duplication of common nodes.
    - Prefer this for persistently-stored unique documents.
  - [`ArcDoc`]: A document that persistently shares its internals, via [`Arc`].
    - Simple to build statically *or* use a builder pattern via [`ArcDocBuilder`],
    - Thread-safe, *but:*
    - May suffer from heap fragmentation.
    - Deduplication and sharing of leaf nodes is lost if serialized and deserialized.
    - Prefer this for persistently-stored shared documents.

- For document consumption, refer to the [`Renderer`] trait and its implementors.
  - The input to [`Renderer`] implementors is formed via [`LayoutEngine`], which is accessed by convenience functions [`Document::as_layout`] and [`Document::as_layout_with`].
  - This crate natively supports plaintext rendering via [`PlaintextRenderer`] and convenience functions [`Document::to_plaintext`] and [`Document::to_plaintext_with`].

- The opt-in feature flags below provide the following:
  - `termcolor`: Enables [`termcolor`](https://docs.rs/termcolor/latest/termcolor/) support for annotations via [`ColorPatch`] and rendering via [`TermcolorRenderer`].
  - `serde`: Enables [`serde`](https://crates.io/crates/serde) support by implementing [`serde::Serialize`] and [`serde::Deserialize`] for supported structs and enums.

### Alternatives

There's quite a few! A non-exhaustive list of existing options:
- [`pretty`](https://crates.io/crates/pretty): The classic. Basically the original Wadler algorithm verbatim.
- [`prettyless`](https://crates.io/crates/prettyless): A fork of `pretty` with (allegedly) more ergonomic usage within Rust.
- [`pprint`](https://crates.io/crates/pprint): *In addition* to being a document constructor, also provides derivable pretty-printing for Rust datatypes. Basically a neater version of [`Debug`], neat!
- [`sparkly`](https://crates.io/crates/sparkly): Built-in terminal and ANSI coloring support.

### Will This Be On [`crates.io`](https://crates.io/)?

Maybe, probably not.

[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
[`Document`]: /src/document.rs
[`RefDoc`]: /src/handle/refdoc.rs
[`RefDocBuilder`]: /src/handle/refdoc.rs
[`Arena`]: https://docs.rs/typed-arena/latest/typed_arena/struct.Arena.html
[`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
[`BoxDoc`]: /src/handle/boxdoc.rs
[`Arc`]: https://doc.rust-lang.org/std/sync/struct.Arc.html
[`ArcDoc`]: /src/handle/arcdoc.rs
[`ArcDocBuilder`]: /src/handle/arcdoc.rs
[`Renderer`]: /src/renderer.rs
[`LayoutEngine`]: /src/layout.rs
[`LayoutSettings`]: /src/layout.rs
[`Document::as_layout`]: /src/document.rs
[`Document::as_layout_with`]: /src/document.rs
[`PlaintextRenderer`]: /src/renderer.rs
[`Document::to_plaintext`]: /src/document.rs
[`Document::to_plaintext_with`]: /src/document.rs
[`Deref<Target = Document<...>>`]: https://doc.rust-lang.org/std/ops/trait.Deref.html
[`ColorPatch`]: /src/renderer.rs
[`TermcolorRenderer`]: /src/renderer.rs
[`serde::Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
[`serde::Deserialize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
[`Debug`]: https://doc.rust-lang.org/std/fmt/trait.Debug.html

License: MIT OR Apache-2.0
