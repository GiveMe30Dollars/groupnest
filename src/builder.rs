use std::{borrow::Cow, collections::HashMap, fmt::Debug, ops::Deref};

use derive_more::{From, Into};
use typed_arena::Arena;

use crate::document::{Break, Document, FlatFragment, GroupPolicy};

/// The notation document format, allocated via arena and taking immutable reference to its children and fragments.
///
/// Due to reliance on arena allocation, this type may not be `Send + Sync`.
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors
///
/// Due to not owning its internals, this type cannot construct itself.
/// It is the responsibility of `DocBuilder` and similar data structures to implement the `Document` smart constructors.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Into)]
pub struct Doc<'s, 'doc, A = ()>(pub &'doc Document<'s, Self, A>);
impl<'s, 'doc, A> Deref for Doc<'s, 'doc, A> {
    type Target = Document<'s, Self, A>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// The interning key for leaf nodes of a `Doc`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LeafKey<'s> {
    Nil,
    Text(FlatFragment<'s>),
    Break(Break<'s>),
    HardLinebreak,
}
impl<'s, 'doc, A> TryFrom<&'doc Document<'s, Doc<'s, 'doc, A>, A>> for LeafKey<'s> {
    type Error = ();
    fn try_from(value: &'doc Document<'s, Doc<'s, 'doc, A>, A>) -> Result<Self, Self::Error> {
        match value {
            Document::Nil => Ok(LeafKey::Nil),
            Document::Text(inner) => Ok(LeafKey::Text(inner.clone())),
            Document::Break(inner) => Ok(LeafKey::Break(inner.clone())),
            Document::HardLinebreak => Ok(LeafKey::HardLinebreak),
            _ => Err(()),
        }
    }
}

/// The builder structure for `Doc`.
///
/// ## Note on [`Document`](crate::document::Document) Smart Constructors
///
/// Due to interning and arena allocation, a `&mut self` parameter is taken for all smart constructors.
///
/// ## Note on [`Debug`](std::fmt::Debug)
///
/// Debug information on the backing arena is omitted because [`typed_arena::Arena`] does not implement Debug.
#[derive(Clone)]
pub struct DocBuilder<'s, 'doc, A> {
    arena: &'doc Arena<Document<'s, Doc<'s, 'doc, A>, A>>,
    intern: HashMap<LeafKey<'s>, &'doc Document<'s, Doc<'s, 'doc, A>, A>>,
}
impl<'s, 'doc, A> Debug for DocBuilder<'s, 'doc, A>
where
    A: Debug,
{
    /// Debug information on the backing arena is omitted because [`typed_arena::Arena`] does not implement Debug.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocBuilder")
            .field("arena", &"[Debug representation hidden]")
            .field("intern", &self.intern)
            .finish()
    }
}

impl<'s, 'doc, A> DocBuilder<'s, 'doc, A> {
    /// Create a new DocBuilder. The backing arena must remain live.
    /// Despite the type parameter `A` defaulting to `()` in `Doc` and `Document`,
    /// it must be specified here.
    /// If you intend to consume the document in the same scope as the builder and arena:
    /// ```
    /// use groupnest::{Arena, DocBuilder};
    /// let arena = Arena::new();
    /// let builder: DocBuilder<'_, '_, ()> = DocBuilder::new(&arena);
    /// ```
    pub fn new(arena: &'doc Arena<Document<'s, Doc<'s, 'doc, A>, A>>) -> Self {
        Self {
            arena,
            intern: HashMap::new(),
        }
    }

    /// For internal usage: allocate and/or intern a document node into the arena.
    /// Hence, the `alloc` closure expected by `Document` is `|inner| self.alloc(inner)`.
    fn alloc<'a>(&'a mut self, doc: Document<'s, Doc<'s, 'doc, A>, A>) -> Doc<'s, 'doc, A> {
        let is_leaf = LeafKey::try_from(&doc);
        // If this is a leaf node, try to find an interned copy, and return that instead.
        if let Ok(leaf_key) = &is_leaf
            && let Some(cached) = self.intern.get(leaf_key)
        {
            return Doc(*cached);
        }
        // Allocate.
        let reference = self.arena.alloc(doc);
        // If this is a leaf node, intern this, which has no copy given the earlier check.
        if let Ok(leaf_key) = is_leaf {
            self.intern.insert(leaf_key, reference);
        }
        Doc(reference)
    }

    /// The smart constructor for a nil node.
    pub fn nil(&mut self) -> Doc<'s, 'doc, A> {
        Document::nil(|doc| self.alloc(doc))
    }
    /// The smart constructor for literal text.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains tabs.
    pub fn from_text(&mut self, payload: impl Into<Cow<'s, str>>) -> Doc<'s, 'doc, A> {
        Document::from_text(payload, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }

    /// The smart constructor for flat text fragments.
    ///
    /// # Panics
    ///
    /// Panics if the payload contains newline sequences or tabs.
    pub fn flat_text(&mut self, payload: impl Into<Cow<'s, str>>) -> Doc<'s, 'doc, A> {
        Document::flat_text(payload, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The smart constructor for break nodes.
    ///
    /// # Panics
    ///
    /// Panics if `flat` contains newline sequences, `broken` does not contain any, and either contain tabs.
    pub fn breaker(
        &mut self,
        flat: impl Into<Cow<'s, str>>,
        broken: impl Into<Cow<'s, str>>,
    ) -> Doc<'s, 'doc, A> {
        Document::breaker(flat, broken, |inner| self.alloc(inner))
            .map_err(|err| panic!("{err}"))
            .unwrap()
    }
    /// The smart constructor for a hard linebreak.
    pub fn hard_linebreak(&mut self) -> Doc<'s, 'doc, A> {
        Document::hard_linebreak(|inner| self.alloc(inner))
    }
    /// The smart constructor for a group with the specified policy.
    pub fn group(&mut self, child: Doc<'s, 'doc, A>, policy: GroupPolicy) -> Doc<'s, 'doc, A> {
        Document::group(child, policy, |inner| self.alloc(inner))
    }
    /// The smart constructor for a grouped sequence.
    pub fn grouped_sequence(
        &mut self,
        children: Vec<Doc<'s, 'doc, A>>,
        policy: GroupPolicy,
    ) -> Doc<'s, 'doc, A> {
        Document::grouped_sequence(children, policy, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence.
    pub fn sequence(&mut self, children: Vec<Doc<'s, 'doc, A>>) -> Doc<'s, 'doc, A> {
        Document::sequence(children, |inner| self.alloc(inner))
    }
    /// The smart constructor for a collection sequence with interspersion.
    pub fn sequence_intersperse_with(
        &mut self,
        children: Vec<Doc<'s, 'doc, A>>,
        separator: Doc<'s, 'doc, A>,
    ) -> Doc<'s, 'doc, A>
    where
        A: Clone,
    {
        Document::sequence_intersperse_with(children, separator, |inner| self.alloc(inner))
    }
    /// The smart constructor for nesting.
    pub fn nest(&mut self, indentation: usize, inner: Doc<'s, 'doc, A>) -> Doc<'s, 'doc, A> {
        Document::nest(indentation, inner, |inner| self.alloc(inner))
    }
    /// The smart constructor for annotations.
    pub fn annotation(&mut self, annotation: A, inner: Doc<'s, 'doc, A>) -> Doc<'s, 'doc, A> {
        Document::annotation(annotation, inner, |inner| self.alloc(inner))
    }
}
