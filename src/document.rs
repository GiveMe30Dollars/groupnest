//! Definition and related types for document representation.
//!
//! Refer to `Document` for explanation.

use std::{
    borrow::Cow,
    ops::{Add, Deref},
};

use thiserror::Error;
use unicode_width::UnicodeWidthStr;

use crate::{
    PlaintextRenderer, RenderAdaptorExt, layout::{LayoutEngine, LayoutSettings}, lines, renderer::RenderError
};

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
pub enum ContainsTab {
    #[error("Contains tab at byte offset {0} of string '{1:?}'.")]
    ContainsTab(usize, String),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum FragmentError {
    #[error("Contains linebreak sequence {0:?} at byte offset {1} of string '{2:?}'.")]
    ContainsLinebreak(String, usize, String),
    #[error(transparent)]
    ContainsTab(#[from] ContainsTab),
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
    /// This method fails if the fragment contains a newline character, identified by the `lines` module, or tabs.
    pub fn new(payload: impl Into<Cow<'s, str>>) -> Result<Self, FragmentError> {
        let inner = payload.into();
        if let Some(index) = inner.find("\t") {
            return Err(ContainsTab::ContainsTab(index, inner.to_string()).into());
        }
        if let Some((index, span)) = lines::next_linebreak(&inner, 0) {
            return Err(FragmentError::ContainsLinebreak(
                inner[index..(index + span)].to_string(),
                index,
                inner.to_string(),
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
    ///
    /// # Errors
    ///
    /// This function fails if the payload contains any tabs.
    pub fn from_text(
        payload: impl Into<Cow<'s, str>>,
    ) -> Result<Vec<TextFragment<'s>>, ContainsTab> {
        let inner: Cow<'s, str> = payload.into();
        if let Some(index) = inner.find('\t') {
            return Err(ContainsTab::ContainsTab(index, inner.to_string()));
        }
        let mut lines_iter = lines::LinesCow::new(inner)
            .map(|line| TextFragment::Text(FlatFragment::new(line).unwrap()))
            .peekable();
        let mut result = Vec::new();
        if let Some(first_line) = lines_iter.next() {
            result.push(first_line);
        } else {
            // There's nothing here.
            return Ok(Vec::new());
        }
        for line in lines_iter {
            result.push(TextFragment::Linebreak);
            // Strip empty lines from being emitted as `TextFragment::text`
            if matches!(&line, TextFragment::Text(fragment) if fragment.width > 0) {
                result.push(line);
            }
        }
        Ok(result)
    }
}

/// The error type for Break construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum BreakNodeInvalid {
    #[error(transparent)]
    FragmentError(#[from] FragmentError),
    #[error("The `broken` payload has no newline sequences.")]
    BrokenPayloadHasNoNewline,
}
impl From<ContainsTab> for BreakNodeInvalid {
    fn from(value: ContainsTab) -> Self {
        BreakNodeInvalid::FragmentError(value.into())
    }
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
        let broken_vec = TextFragment::from_text(broken)?;
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

/// An enum regarding whether the contents of a [`Document::Group`] node can be displayed flat.
///
/// Used for [`LayoutSettings`] configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakStatus {
    /// This group contains one or more [`Document::HardLinebreak`], or [`Document::Group`] with policy [`GroupPolicy::ForceBreak`].
    /// This group *must* be displayed broken, regardless of current policy.
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

/// The group policy governing how a [`Document::Group`] node interacts with layout.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GroupPolicy {
    /// The default policy.
    /// - If the flat length is no more than a minimum threshold within width constraints, display flat.
    /// - If the flat length does not exceed the remaining space in this line, display flat.
    /// - Else, display broken.
    #[default]
    Normal,
    /// Forces this group to always display flat if and only if it contains no forced linebreaks.
    ///
    /// In other words, this group always succeeds unless the child has:
    /// - One or more [`Document::HardLinebreak`] nodes.
    /// - One or more inner [`Document::Group`] with the [`GroupPolicy::ForceBreak`] policy
    ///   where the inner child has influencable [`Document::Break`] nodes.
    ///
    /// This policy, when active, overrides the policy of enclosed group nodes,
    /// causing the entire notation within to be displayed flat.
    /// Applying this policy onto a group may cause otherwise-adherent groups to violate strict width constraints.
    FlatIfPossible,
    /// Forces this group to always display broken. Inner groups may display broken or inline.
    /// This policy is infallible.
    ForceBreak,
}

/// A sequence containing an immutable collection of children.
///
/// ## Why Is This Not [`Vec`]?
///
/// Wadler's algorithm relies on the calculation of the flat length of a [`Document`].
///
/// To prevent quadratic complexity during layout generation for this calculation,
/// these values are cached upon construction for collections,
/// leading to time complexity linear to the depth of the document tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sequence<D> {
    children: Box<[D]>,
    pub(crate) status: BreakStatus,
    pub(crate) layout_mode_observable: bool,
}
impl<D> Sequence<D> {
    /// Provides an immutable reference to the children collection.
    pub fn children(&self) -> &[D] {
        &self.children
    }
    /// Consumes self, returning the inner children collection.
    pub fn into_children(self) -> Box<[D]> {
        self.children
    }
}
impl<'s, D, A> Sequence<D>
where
    D: Deref<Target = Document<'s, D, A>>,
{
    /// Constructs a new sequence collection.
    pub fn new(children: Box<[D]>) -> Self {
        let status = children
            .iter()
            .fold(BreakStatus::FlatLength(0), |acc, child| {
                acc + child.break_status()
            });
        let layout_mode_observable = children
            .iter()
            .any(|child| child.layout_mode_observable(GroupPolicy::Normal));
        Sequence {
            children,
            status,
            layout_mode_observable,
        }
    }
}

/// The raw notation document format of a Wadler-style pretty printer.
///
/// Parameterized over:
/// - `'s`: the lifetime of owned or borrowed string fragments, which are of type [`Cow<'s, str>`].
/// - `D`: the type of children. This should be a fixed-point reference or smart pointer, wrapped in a Rust newtype.  
///   `D` should implement [`Deref<Target = Document<..>>`].  
///   Provided implementors include:
///   - [`RefDoc`](crate::RefDoc) for arena-backed allocation and construction, with sharing of leaf nodes.
///   - [`OwnedDoc`](crate::OwnedDoc) for directly-owned [`Box`]'ed values.
/// - `A`: the type of annotations. Defaults to unit type `()`.
///
/// Some of the variants use opaque datatypes with accessor functions to maintain internal invariants.
/// Generally, a `Document` should be treated as immutable upon construction.
/// Subsequent passes may read from the existing `Document` and generate a modified copy if desired.
///
/// Do not construct this type directly.
///
/// ## Note on Smart Constructors
///
/// This document type provides smart constructors that perform local structural transformations.
/// - An allocation closure `alloc` of `FnMut(Self) -> D` is required.
/// - The input and output types of these smart constructors are of type `D`.
/// - Document builders and/or wrapper types are encouraged to shadow the smart constructors with morally-equivalent methods:
///   <details>
///   <summary><i> Click to expand </i></summary>
///
///   ```ignore
///   /// The smart constructor for a nil node.
///   pub fn nil() -> Self;
///   /// The smart constructor for literal text.
///   ///
///   /// # Panics
///   ///
///   /// Panics if the payload contains tabs.
///   pub fn from_text(payload: impl Into<Cow<'s, str>>) -> Self;
///   /// The non-panicking smart constructor for literal text.
///   pub fn from_text_(payload: impl Into<Cow<'s, str>>) -> Result<Self, ContainsTab>;
///   /// The smart constructor for flat text fragments.
///   ///
///   /// # Panics
///   ///
///   /// Panics if the payload contains newline sequences or tabs.
///   pub fn flat_text(payload: impl Into<Cow<'s, str>>) -> Self;
///   /// The non-panicking smart constructor for flat text fragments.
///   pub fn flat_text_(payload: impl Into<Cow<'s, str>>) -> Result<Self, FragmentError>;
///   /// The smart constructor for break nodes.
///   ///
///   /// # Panics
///   ///
///   /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
///   pub fn breaker(flat: impl Into<Cow<'s, str>>, broken: impl Into<Cow<'s, str>>) -> Self;
///   /// The non-panicking smart constructor for break nodes.
///   pub fn breaker_(flat: impl Into<Cow<'s, str>>, broken: impl Into<Cow<'s, str>>) -> Result<Self, BreakNodeInvalid>;
///   /// The smart constructor for a hard linebreak
///   pub fn hard_linebreak() -> Self;
///
///   /// The smart constructor for a group with the specified policy.
///   pub fn group(child: Self, policy: GroupPolicy) -> Self;
///   /// The smart constructor for a grouped sequence.
///   pub fn grouped_sequence(children: Vec<Self>, policy: GroupPolicy) -> Self;
///   /// The smart constructor for a collection sequence.
///   pub fn sequence(children: Vec<Self>) -> Self;
///   /// The smart constructor for a collection sequence with interspersion.
///   pub fn sequence_intersperse_with(children: Vec<Self>, separator: Self) -> Self where Self : Clone;
///   /// The smart constructor for nesting.
///   pub fn nest(indentation: usize, inner: Self) -> Self;
///
///   /// The smart constructor for annotations.
///   pub fn annotation(annotation: A, inner: Self) -> Self;
///   ```
///
///   </details>
///
///   This would likely require defining and applying the `alloc` closure, which in some cases is just [`Into::into`].
///
///
/// ## Note on Canonical Representation
///
/// The builders exposed here do not inherently build canonical documents.
/// In other words, [`Eq`] functions on documents for structural equivalence, *not* semantic equivalence.
///
/// There are many ways to express equivalent documents,
/// and while a best-effort attempt is made to reduce or flatten equivalent forms,
/// some transformations are not possible without global inspection and reconstruction of the document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Document<'s, D, A = ()> {
    /// A no-op node. This node displays as `""`, carries no meaning, and builders will attempt to eliminate it.
    Nil,
    /// Text that *must* be displayed flat. Contains no newlines.
    Text(FlatFragment<'s>),
    /// A breaking node, which renders differently based on whether the current layout mode is flat or broken.
    /// Also used to express soft linebreaks with the form `Break(flat = "", broken = "\n")`.
    ///
    /// Respects indentation added by enclosing `Nest` nodes.
    Break(Break<'s>),
    /// A hard linebreak, which must appear in the final rendering.
    ///
    /// Respects indentation added by enclosing `Nest` nodes.
    HardLinebreak,
    /// A group, which introduces a layout decision point.
    ///
    /// A group node will render based on its policy, the default being to render flat if possible and broken otherwise.
    Group(GroupPolicy, D),
    /// A sequence containing a collection of children.
    /// 
    /// This node by itself does *not* introduce a layout decision point. Refer to [`Document::Group`].
    /// A common usage pattern is `Group(policy = _, Sequence(...))` for introducing a sequence with a decisioin point.
    Sequence(Sequence<D>),
    /// A node that, if in broken layout mode, will add indentation to its child.
    ///
    /// The additional indentation applies only on logical newlines;
    /// the current line will be unaffected by entering or exiting of a `Nest` scope if it already contains non-padding text.
    Nest(usize, D),
    /// An annotation.
    /// 
    /// The layout algorithm assumes that annotations do not affect layout decisions,
    /// and defers rendering choices to respective [`Renderer`](crate::Renderer) implementors.
    Annotation(Box<A>, D),
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
            Document::Group(policy, child) => {
                if matches!(policy, GroupPolicy::ForceBreak)
                    && child.layout_mode_observable(GroupPolicy::ForceBreak)
                {
                    BreakStatus::MustBreak
                } else {
                    child.break_status()
                }
            }
            Document::Sequence(sequence) => sequence.status,
            Document::Nest(indent, child) => {
                BreakStatus::FlatLength(*indent) + child.break_status()
            }
            Document::Annotation(_, child) => child.break_status(),
        }
    }
    /// Returns whether a layout mode change is observable on this node.
    /// Basically: whether a node contains influencable `Break` children.
    ///
    /// This is used as a helper for `break_status()` along with determining whether a `FlatIfPossible` applies.
    pub(crate) fn layout_mode_observable(&self, policy: GroupPolicy) -> bool {
        match self {
            Document::Break(_) => true,
            Document::Nil | Document::Text(_) | Document::HardLinebreak => false,

            // Since `Group` introduces an independent layout decision,
            // The current layout mode is not observable in the child,
            // UNLESS it is `FlatIfPossible -> Normal`, which overrides the inner decision.
            // This adds coupling to the layout engine implementation,
            // but since this is called in `break_status()` without the override (using `Normal`),
            // it returns the right answers while exposing a canonization opportunity in `Group` smart construction.
            Document::Group(inner_policy, child) => {
                if matches!(
                    (policy, inner_policy),
                    (GroupPolicy::FlatIfPossible, GroupPolicy::Normal)
                ) {
                    child.layout_mode_observable(GroupPolicy::FlatIfPossible)
                } else {
                    false
                }
            }

            Document::Nest(_, child) | Document::Annotation(_, child) => {
                child.layout_mode_observable(policy)
            }
            Document::Sequence(sequence) => sequence.layout_mode_observable,
        }
    }

    /// Borrows the notation document to produce a layout engine, using default settings.
    pub fn as_layout<'doc>(&'doc self) -> LayoutEngine<'s, 'doc, D, A> {
        LayoutEngine::new(self)
    }
    /// Borrows the notation document to produce a layout engine with the specified settings.
    pub fn as_layout_with<'doc>(&'doc self, settings: LayoutSettings) -> LayoutEngine<'s, 'doc, D, A> {
        LayoutEngine::with_settings(self, settings)
    }

    /// Borrows the notation document to produce a plaintext document, using default settings, with annotations stripped.
    pub fn to_plaintext(&self) -> Result<String, RenderError> {
        PlaintextRenderer::render_to_string(self.as_layout().strip_annotation())
    }
    /// Borrows the notation document to produce a plaintext document with the specified settings, with annotations stripped.
    pub fn to_plaintext_with(&self, settings: LayoutSettings) -> Result<String, RenderError> {
        PlaintextRenderer::render_to_string(self.as_layout_with(settings).strip_annotation())
    }

    // Smart Construction Methods

    /// The 'smart' constructor for a nil node.
    pub fn nil<F>(mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        alloc(Document::Nil)
    }
    /// The smart constructor for literal text.
    /// Splits a given text along its newline sequences, returning one of the following node types:
    /// - `Nil` for nothing,
    /// - `Text` for flat text fragments,
    /// - `HardLinebreak` for newline sequences, or
    /// - `Sequence` consisting of multiple of the above.
    pub fn from_text<F>(payload: impl Into<Cow<'s, str>>, mut alloc: F) -> Result<D, ContainsTab>
    where
        F: FnMut(Self) -> D,
    {
        let fragments = TextFragment::from_text(payload)?;
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
        Ok(Self::sequence(children, alloc))
    }
    /// The smart constructor for flat text.
    /// Eliminates empty payloads, returning `Nil` instead.
    ///
    /// # Errors
    ///
    /// Fails if payload contains newline sequences.
    /// Shadowing methods may choose to `panic!()` instead for more ergonomic usage.
    pub fn flat_text<F>(payload: impl Into<Cow<'s, str>>, mut alloc: F) -> Result<D, FragmentError>
    where
        F: FnMut(Self) -> D,
    {
        let flat = FlatFragment::new(payload)?;
        Ok(alloc(if flat.width == 0 {
            Document::Nil
        } else {
            Document::Text(flat)
        }))
    }
    /// The smart constructor for break nodes.
    ///
    /// # Errors
    ///
    /// Fails if:
    /// - Flat payload contains any newline sequences.
    /// - Broken payload does *not* contain any newline sequences.
    ///
    /// Shadowing methods may choose to `panic!()` instead for more ergonomic usage.
    pub fn breaker<F>(
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
        mut alloc: F,
    ) -> Result<D, BreakNodeInvalid>
    where
        F: FnMut(Self) -> D,
    {
        Break::new(flat, broken).map(|inner| alloc(Document::Break(inner)))
    }
    /// The 'smart' constructor for a hard linebreak.
    pub fn hard_linebreak<F>(mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        alloc(Document::HardLinebreak)
    }

    /// The smart constructor for a group with the specified policy.
    pub fn group<F>(child: D, policy: GroupPolicy, mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        // While I previously attempted elimination of outer group depending on policy,
        // Treating them uniformly was just easier and less prone to error should the layout engine change.
        // To my knowledge, only (outer, inner) == (FlatIfPossible, Normal) required special treatment,
        // as the inner policy of `Normal` is overriden by FlatIfPossible.
        // Every other case preferred the inner existing policy.
        alloc(Document::Group(policy, child))
    }
    /// The smart constructor for a grouped collection sequence with the specified policy.
    pub fn grouped_sequence<F>(children: Vec<D>, policy: GroupPolicy, mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        let sequence = Self::sequence(children, &mut alloc);
        Self::group(sequence, policy, alloc)
    }

    /// The smart constructor for a collection sequence.
    /// Note that `Sequence` nodes do not automatically introduce layout decisions;
    /// use `Group` and its associated smart constructors.
    ///
    /// - Eliminates `Nil` children, and if none remain returns `Nil` itself.
    /// - Prevents construction of `Sequence` nodes with exactly one child, returning it instead.
    pub fn sequence<F>(children: Vec<D>, mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        let mut children = children
            .into_iter()
            .flat_map(|elem| match *elem {
                Document::Nil => Vec::new(),
                _ => vec![elem],
            })
            .collect::<Vec<_>>();
        if children.is_empty() {
            Document::nil(alloc)
        } else if children.len() == 1 {
            children.pop().unwrap()
        } else {
            alloc(Document::Sequence(Sequence::new(
                children.into_boxed_slice(),
            )))
        }
    }
    /// The smart constructor for a collection sequence, in which each child is interspersed with a separator.
    /// - `Nil` nodes are filtered out before this interspersion to avoid duplicates.
    /// - `Document::sequence(..)` optimizations apply.
    pub fn sequence_intersperse_with<F>(children: Vec<D>, separator: D, alloc: F) -> D
    where
        F: FnMut(Self) -> D,
        D: Clone,
    {
        if matches!(*separator, Document::Nil) {
            return Self::sequence(children, alloc);
        }
        let children = children
            .into_iter()
            .filter_map(|elem| match *elem {
                Document::Nil => None,
                _ => Some(elem),
            })
            .collect::<Vec<_>>();
        let mut interspersed = Vec::new();
        for child in children {
            if interspersed.is_empty() {
                interspersed.push(child);
            } else {
                interspersed.push(separator.clone());
                interspersed.push(child);
            }
        }
        Self::sequence(interspersed, alloc)
    }

    /// The smart constructor for nesting.
    /// - Propagates `Nil` nodes.
    pub fn nest<F>(indentation: usize, inner: D, mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        if matches!(*inner, Document::Nil) {
            Document::nil(alloc)
        } else {
            alloc(Document::Nest(indentation, inner))
        }
    }

    /// The `smart` constructor for annotations.
    pub fn annotation<F>(annotation: A, inner: D, mut alloc: F) -> D
    where
        F: FnMut(Self) -> D,
    {
        alloc(Document::Annotation(Box::new(annotation), inner))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::OwnedDoc;
    #[test]
    fn group_nil_no_break() {
        let data: OwnedDoc<()> = OwnedDoc::group(OwnedDoc::nil(), GroupPolicy::ForceBreak);
        assert!(!data.layout_mode_observable(GroupPolicy::Normal));
        assert_eq!(data.break_status(), BreakStatus::FlatLength(0));
    }
    #[test]
    fn observable_override() {
        let data: OwnedDoc<()> = OwnedDoc::group(
            OwnedDoc::breaker("somebody", "once\ntold\nme"),
            GroupPolicy::Normal,
        );
        assert!(data.layout_mode_observable(GroupPolicy::FlatIfPossible));
        assert_eq!(data.break_status(), BreakStatus::FlatLength(8));
    }
}
