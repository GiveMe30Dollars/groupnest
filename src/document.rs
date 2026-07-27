use std::{
    borrow::Cow,
    ops::{Add, Deref},
};

use thiserror::Error;
use unicode_width::UnicodeWidthStr;

use crate::{layout::LayoutEngine, lines};

/// An flat text fragment.
/// The payload is asserted to contain no linebreaks upon construction.
///
/// This type is not intended to be constructed externally.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FlatFragment<'s> {
    inner: Cow<'s, str>,
    pub(crate) width: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ContainsLinebreak {
    #[error("Contains linebreak sequence {0:?} at {1}")]
    ContainsLinebreak(String, usize),
}
impl<'s> FlatFragment<'s> {
    /// Provides an immutable reference into the inner payload.
    pub fn inner(&self) -> &str {
        &self.inner
    }
    /// Consumes self and returns its inner payload.
    pub fn into_inner(self) -> Cow<'s, str> {
        self.inner
    }
    /// Constructs an flat fragment, given a raw fragment that can be taken as a string reference.
    ///
    /// # Errors
    ///
    /// This method fails if the fragment contains a newline character, identified by the `lines` module.
    pub fn new(payload: impl Into<Cow<'s, str>>) -> Result<Self, ContainsLinebreak> {
        let inner = payload.into();
        if let Some((index, span)) = lines::next_linebreak(&inner, 0) {
            return Err(ContainsLinebreak::ContainsLinebreak(
                inner[index..(index + span)].to_string(),
                index,
            ));
        }
        let width = UnicodeWidthStr::width(inner.as_ref());
        Ok(Self { inner, width })
    }
}

/// A text fragment, which is either a continuous flat fragment or a newline.
/// Used in `Break` notation nodes.
///
/// This type is not intended to be constructed externally.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextFragment<'s> {
    Text(FlatFragment<'s>),
    Linebreak,
}
impl<'s> TextFragment<'s> {
    /// Given raw text, segmentate it along newline separators and return zero, one or more text fragments.
    /// Unlike InlineFragment<'s>, this function is infallible.
    pub fn from_text(payload: impl Into<Cow<'s, str>>) -> Vec<TextFragment<'s>> {
        let inner: Cow<'s, str> = payload.into();
        let mut lines_iter = lines::LinesCow::new(inner)
            .map(|line| TextFragment::Text(FlatFragment::new(line).unwrap()))
            .peekable();
        let mut result = Vec::new();
        if let Some(first_line) = lines_iter.next() {
            result.push(first_line);
        } else {
            // There's nothing here.
            return Vec::new();
        }
        for line in lines_iter {
            result.push(TextFragment::Linebreak);
            // Strip empty lines from being emitted as `TextFragment::text`
            if matches!(&line, TextFragment::Text(fragment) if fragment.width > 0) {
                result.push(line);
            }
        }
        result
    }
}

/// The error type for Break construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum BreakNodeInvalid {
    #[error(transparent)]
    ContainsLinebreak(#[from] ContainsLinebreak),
    #[error("The `broken` payload has no newline sequences.")]
    BrokenPayloadHasNoNewline,
}

/// A breaking node, which renders differently based on whether the current layout mode is flat or broken.
/// Also used to express soft linebreaks.
/// Respects indentation added by enclosing `Nest` nodes.
///
/// Invariants:
/// - The `broken` field must contain at least one linebreak.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Break<'s> {
    flat: FlatFragment<'s>,
    broken: Box<[TextFragment<'s>]>,
}
impl<'s> Break<'s> {
    /// Returns an immutable reference into the `flat` field.
    pub fn flat(&self) -> &FlatFragment<'s> {
        &self.flat
    }
    /// Returns an immutable reference into the `broken` field.
    pub fn broken(&self) -> &[TextFragment<'s>] {
        &self.broken
    }
    /// Consumes self and returns its constitutent components.
    pub fn into_inner(self) -> (FlatFragment<'s>, Box<[TextFragment<'s>]>) {
        (self.flat, self.broken)
    }
}
impl<'s> Break<'s> {
    pub fn new(
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<Self, BreakNodeInvalid> {
        let unbroken_fragment = FlatFragment::new(flat)?;
        let broken_vec = TextFragment::from_text(broken);
        if broken_vec
            .iter()
            .any(|elem| matches!(elem, TextFragment::Linebreak))
        {
            Ok(Self {
                flat: unbroken_fragment,
                broken: broken_vec.into_boxed_slice(),
            })
        } else {
            Err(BreakNodeInvalid::BrokenPayloadHasNoNewline)
        }
    }
}

/// An enum regarding whether the contents of a Group node can be displayed flat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakStatus {
    /// This group contains one or more `HardLinebreaks`. This group *must* be displayed broken, regardless of policy.
    MustBreak,
    /// This group has a minimal flat length. Whether it is displayed flat or broken depends on the group policy.
    FlatLength(usize),
}
impl Add for BreakStatus {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (BreakStatus::FlatLength(length), BreakStatus::FlatLength(rhs_length)) => {
                BreakStatus::FlatLength(length + rhs_length)
            }
            // Any combination of MustBreak results in MustBreak.
            _ => BreakStatus::MustBreak,
        }
    }
}

/// The group policy governing how a group node interacts with layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GroupPolicy {
    /// The default policy.
    /// - If the unbroken length of its children is no more than a minimum threshold within relaxed constraints, display flat.
    /// - If the unbroken length of its children does not exceed the remaining space in this line, display flat.
    /// - Else, display broken.
    #[default]
    Default,
    /// Forces this group to always display flat if and only if it contains no forced linebreaks.
    /// This occurs when this group's children does not contains any of the following:
    /// - One or more `HardLinebreak` nodes.
    /// - One or more groups with the `ForceBreak` policy.
    ///
    /// This policy, when active, overrides the policy of enclosed group nodes, causing the entire notation within to be displayed flat.
    /// Applying this policy onto a group may cause otherwise-adherent groups to violate strict width constraints.
    ForceFlat,
    /// Forces this group to always display broken.
    ForceBreak,
}

/// A group containing multiple children nodes.
/// A group node will render based on its policy, the default being to render flat if possible and broken otherwise.
///
/// Invariants:
/// - `children` must be immutable due to caching of overall break status.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Group<D> {
    children: Box<[D]>,
    policy: GroupPolicy,
    pub(crate) status: BreakStatus,
}
impl<D> Group<D> {
    pub fn children(&self) -> &[D] {
        &self.children
    }
    pub fn policy(&self) -> GroupPolicy {
        self.policy
    }
    pub fn into_children(self) -> Box<[D]> {
        self.children
    }
}
impl<'s, D, A> Group<D>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    pub fn new(children: Box<[D]>) -> Self {
        Self::with_policy(children, GroupPolicy::Default)
    }
    pub fn with_policy(children: Box<[D]>, policy: GroupPolicy) -> Self {
        let status = children.iter().fold(
            if policy == GroupPolicy::ForceBreak {
                BreakStatus::MustBreak
            } else {
                BreakStatus::FlatLength(0)
            },
            |acc, child| acc + child.break_status(),
        );
        Group {
            children,
            policy,
            status,
        }
    }
}

/// The raw notation document format of a Wadler-style pretty printer.
///
/// Parameterized over:
/// - `'s`: the lifetime of owned or borrowed string fragments, which are of type `Cow<'s, str>`.
/// - `D`: the type of children. This should be a fixed-point reference or smart pointer, wrapped in a Rust newtype.
/// - `A`: the type of annotations. Defaults to unit type `()`.
///
/// Some of the variants use opaque datatypes with accessor functions to maintain internal invariants.
/// Generally, a `Document` should be treated as immutable upon construction.
/// Subsequent passes may read from the existing `Document` and generate a modified copy if desired.
///
/// ## Note on Smart Constructors
///
/// This document type provides smart constructors that perform local structural transformations.
/// - An allocation closure of `impl FnMut(Self) -> D` may be required.
/// - The implementation in `Document` take children of type `D` for nodes with children.
///   This may make direct usage inconvenient.
/// - Document builders and/or wrapper types are encouraged to shadow the smart constructors with morally-equivalent methods:
///   ```
///   pub fn nil() -> Self;
///   pub fn from_text<F>(payload: impl Into<Cow<'static, str>>) -> Self;
///   pub fn flat_text(payload: impl Into<Cow<'static, str>>) -> Result<Self, ContainsLinebreak>;
///   pub fn breaker(flat: impl Into<Cow<'s, str>>, broken: impl Into<Cow<'s, str>>) -> Result<Self, BreakNodeInvalid>;
///   pub fn hard_linebreak() -> Self;
///   pub fn group<F>(children: Vec<Self>) -> Self;
///   pub fn group_with_policy<F>(children: Vec<Self>, policy: GroupPolicy) -> Self;
///   pub fn nest<F>(indentation: usize, inner: Self) -> Self;
///   pub fn annotation<F>(annotation: A, inner: Self) -> Self;
///   ```
///   This would likely only require defining and applying the `alloc` closure, which in some cases is just `Into::into`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Document<'s, D, A = ()> {
    /// A no-op node. This node displays as "", carries no meaning, and builders will attempt to eliminate it
    /// unless enclosed by an annotation.
    Nil,
    /// Text that *must* be displayed flat.
    Text(FlatFragment<'s>),
    /// A breaking node, which renders differently based on whether the current layout mode is flat or broken.
    /// Also used to express soft linebreaks.
    /// Respects indentation added by enclosing `Nest` nodes.
    Break(Break<'s>),
    /// A hard linebreak.
    /// Respects indentation added by enclosing `Nest` nodes.
    HardLinebreak,
    /// A group containing multiple children nodes.
    /// A group node will render based on its policy, the default being to render flat if possible and broken otherwise.
    Group(Group<D>),
    /// A node that, if in broken layout mode, will add indentation to its child.
    Nest(usize, D),
    /// An annotation. The layout engine assumes these do not affect layout choices,
    /// and defers rendering choices to respective Renderer implementors.
    Annotation(A, D),
}
impl<'s, D, A> Document<'s, D, A>
where
    D: Deref<Target = Self>,
{
    /// Returns the break status of this node.
    pub(crate) fn break_status(&self) -> BreakStatus {
        match self {
            Document::Nil => BreakStatus::FlatLength(0),
            Document::Text(fragment) => BreakStatus::FlatLength(fragment.width),
            Document::Break(inner) => BreakStatus::FlatLength(inner.flat().width),
            Document::HardLinebreak => BreakStatus::MustBreak,
            Document::Group(group) => {
                // We can calculate this recursively, but caching it upon group node construction avoids quadratic complexity.
                group.status
            }
            Document::Nest(indent, child) => {
                BreakStatus::FlatLength(*indent) + child.break_status()
            }
            Document::Annotation(_, child) => child.break_status(),
        }
    }

    /// Borrows the notation document to produce a layout engine, with default settings.
    pub fn as_layout<'a>(&'a self) -> LayoutEngine<'s, D, A>
    where
        'a: 's,
    {
        LayoutEngine::new(self)
    }

    // Smart Construction Methods

    /// The 'smart' constructor for a nil node.
    pub fn nil() -> Self {
        Document::Nil
    }
    /// The smart constructor for literal text.
    /// Splits a given text along its newline sequences, returning one of the following node types:
    /// - `Nil` for nothing,
    /// - `Text` for flat text fragments,
    /// - `HardLinebreak` for newline sequences, or
    /// - `Group` consisting of several `Text` and `HardLinebreak`.
    ///
    /// Requires `D`-type allocator.
    pub fn from_text<F>(payload: impl Into<Cow<'static, str>>, mut alloc: F) -> Self
    where
        F: FnMut(Self) -> D,
    {
        let fragments = TextFragment::from_text(payload);
        let children = fragments
            .into_iter()
            .map(|elem| {
                alloc(match elem {
                    TextFragment::Text(flat) => {
                        if flat.width != 0 {
                            Document::Text(flat)
                        } else {
                            Document::Nil
                        }
                    }
                    TextFragment::Linebreak => Document::HardLinebreak,
                })
            })
            .collect::<Vec<_>>();
        Self::group(children)
    }
    /// The smart constructor for flat text.
    /// Eliminates empty payloads, returning `Nil` instead.
    ///
    /// # Errors
    ///
    /// Fails if payload contains newline sequences.
    pub fn flat_text(payload: impl Into<Cow<'static, str>>) -> Result<Self, ContainsLinebreak> {
        let flat = FlatFragment::new(payload)?;
        if flat.width == 0 {
            Ok(Document::Nil)
        } else {
            Ok(Document::Text(flat))
        }
    }
    /// The smart constructor for break nodes.
    ///
    /// # Errors
    ///
    /// Fails if:
    /// - Flat payload contains any newline sequences.
    /// - Broken payload does *not* contain any newline sequences.
    pub fn breaker(
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<Self, BreakNodeInvalid> {
        Break::new(flat, broken).map(|inner| Document::Break(inner))
    }
    /// The 'smart' constructor for a hard linebreak.
    pub fn hard_linebreak() -> Self {
        Document::HardLinebreak
    }

    /// The smart constructor for a group, using the default policy.
    /// - Eliminates `Nil` children, and if none remain returns `Nil` itself.
    pub fn group(children: Vec<D>) -> Self {
        Self::group_with_policy(children, GroupPolicy::Default)
    }
    /// The smart constructor for a group with a specified given policy.
    /// - Eliminates `Nil` children, and if none remain returns `Nil` itself.
    pub fn group_with_policy(children: Vec<D>, policy: GroupPolicy) -> Self {
        let children = children
            .into_iter()
            .filter_map(|elem| match *elem {
                Document::Nil => None,
                _ => Some(elem),
            })
            .collect::<Vec<_>>();
        if children.is_empty() {
            Document::Nil
        } else {
            // While I previously attempted elimination of outer group depending on policy,
            // Treating them uniformly was just easier and less prone to error should the layout engine change.
            // To my knowledge, only (outer, inner) == (ForceFlat, Default) required special treatment,
            // as the inner policy of `Default` is overriden by ForceFlat.
            // Every other case preferred the inner existing policy.
            Document::Group(Group::with_policy(children.into_boxed_slice(), policy))
        }
    }

    /// The smart constructor for nesting.
    /// - Propagates `Nil` nodes.
    pub fn nest(indentation: usize, inner: D) -> Self {
        if matches!(*inner, Document::Nil) {
            Document::Nil
        } else {
            Document::Nest(indentation, inner)
        }
    }

    /// The `smart` constructor for annotation.
    pub fn annotation(annotation: A, inner: D) -> Self {
        Document::Annotation(annotation, inner)
    }
}
