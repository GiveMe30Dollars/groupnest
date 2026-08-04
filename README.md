# `groupnest`

Yet another Wadler-style pretty printer, for fun and profit!

## Why Does This Exist?

***One.*** I wanted this for internal use for my compiler project `hasklite` (a Haskell subset), and thought this would be a fun diversion.

***Two.*** Existing crates, while mature in their own right, were unsuited for my usage or just had minor inconveniences that irked me.

In particular:  
1. Follows the original Wadler algebraic formulation, in which:
    - Concatenation is pairwise, leading to deeply-nested document trees.
    - Multiple concatenation options exist.
2. Redundancies in the node types available to the user.
    - Multiple linebreak node types, along with conditionally-displaying text (in addition to optional newlines).
3. Opaque, uninspectable document types.
4. Documentation for usage best described as Spartan, assuming that you have already read the Wadler papers, and then some.

In contrast, this implementation:  
1. Follows an alternate formulation by the Gleam language team, where:
    - Sequences are flattened boxed slices, reducing tree depth.
    - Concatenation is not primitive, but can be done via nested `Sequence` nodes.
      Callers are encouraged to collect their children before document construction.
2. Minimal node types.
    - Eight [`Document`] node types, with each having little to no overlap for semantics and operations.
    - `HardLinebreak`, which mandates a newline in the final output.
    - `Break`, augmented to support additional arbitrary text in its broken payload.
3. Pattern-matchable documents, with most (but not all) of the contents inspectible and deconstructible.
4. Hopefully better documentation.

## Should You Use This?

Probably not. There's quite a few existing options:
- `pretty`: The classic, the pioneer. `prettyless` is a fork of this.
- `pprint`: Derivable pretty-printing for Rust datatypes. Basically a neater version of `Debug`, neat!
- `tiny_pretty`, `pretty_print`: Minimal implementations.
- `sparkly`: Built-in ANSI coloring support. *Any* pretty printer supporting arbitrary annotations can also do this.

This is not an exhaustive list; you can probably find much more!

## Okay, What If I Do Anyways?

- For document construction, refer to the Wadler [`Document`] representation.
- For document consumption, refer to the [`Renderer`] trait and its implementors.
  - The input to [`Renderer`] implementors is formed via [`LayoutEngine`], which is accessed by convenience functions [`Document::as_layout`] and [`Document::as_layout_with`].
  - This crate natively supports plaintext rendering via [`PlaintextRenderer`] and convenience functions [`Document::to_plaintext`] and [`Document::to_plaintext_with`].
  - The optional feature flag `termcolor` enables [`termcolor`](https://docs.rs/termcolor/latest/termcolor/) support for annotations via [`ColorPatch`] and rendering via [`TermcolorRenderer`].

## Will This Be On `crates.io`?

Maybe, probably not.