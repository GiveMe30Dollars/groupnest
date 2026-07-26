use std::{
    borrow::Cow,
    ops::{Add, Deref},
};

use derive_more::{Deref, DerefMut, From, Into};
use thiserror::Error;
use unicode_width::UnicodeWidthStr;

use crate::{
    layout::LayoutEngine,
    lines
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
    pub fn text(payload: impl Into<Cow<'s, str>>) -> Vec<TextFragment<'s>> {
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
            result.push(line);
        }
        result
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
        unbroken: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Result<Self, ContainsLinebreak> {
        let unbroken_fragment = FlatFragment::new(unbroken)?;
        let broken_vec = TextFragment::text(broken);
        Ok(Self {
            flat: unbroken_fragment,
            broken: broken_vec.into_boxed_slice(),
        })
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
    /// This policy overrides the policy of enclosed group nodes, causing the entire notation within to be displayed flat.
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
/// - `'s`: the type of string fragments.
/// - `D`: the type of children. This should be a fixed-point reference or smart pointer, wrapped in a Rust newtype.
/// - `A`: the type of annotations. Defaults to unit type `()`.
///
/// Some of the variants use opaque datatypes with accessor functions to maintain internal invariants.
/// Generally, a `Document` should be treated as immutable upon construction.
/// Subsequent passes may read from the existing `Document` and generate a modified copy if desired.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Document<'s, D, A = ()> {
    /// A no-op node. This node displays as "", carries no meaning, and builders will attempt to eliminate it.
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
    pub fn as_layout(&'s self) -> LayoutEngine<'s, D, A> {
        LayoutEngine::new(self)
    }
}

/// The notation document format,
/// allocated via arena and taking immutable reference to its children and fragments.
/// Due to reliance on arena allocation, this type may not be `Send + Sync`.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, From, Into)]
pub struct Doc<'s, 'doc, A = ()>(Document<'s, &'doc Doc<'s, 'doc, A>, A>);

/// The notation document format, owning all of its children and fragments.
/// This type is `Send + Sync` if the annotation type `A` is `Send + Sync`,
/// but may suffer from memory fragmentation.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, DerefMut, From, Into)]
pub struct OwnedDoc<A = ()>(Document<'static, Box<OwnedDoc<A>>, A>);

const _: () = {
    #[allow(unused)]
    use std::rc::Rc;
    const fn assert_send<T: Send + Sync>() {}
    assert_send::<OwnedDoc>();
    assert_send::<OwnedDoc<String>>();
};
