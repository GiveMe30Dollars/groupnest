use thiserror::Error;

/// A data structure representing events to be consumed by a Renderer when streaming pretty printed text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderEvent<'s, A = ()> {
    /// Print the following text, as is. Includes its width.
    /// This text fragment may not contain newline sequences.
    Text(usize, &'s str),
    /// Insert a newline character.
    Linebreak,
    /// Insert whitespace padding of the following character width.
    Padding(usize),
    /// Marks the beginning of the scope of the following annotation. Follow stack semantics.
    PushAnnotation(A),
    /// Marks the end of the scope of the most recent annotation. Follow stack semantics.
    PopAnnotation,
    /// A signalling error. Emitted in the following situations:
    /// - After a complete line where a `Strict` width constraint is violated.
    Error(LayoutError),
}

/// Errors resulting from layout rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum LayoutError {
    #[error(
        "Line {line_num} has a width of {line_width} characters, exceeding the maximum width {max_width}"
    )]
    WidthExceeded {
        /// One-indexed.
        line_num: usize,
        max_width: usize,
        line_width: usize,
    },
}

/// An iterator adaptor for mapping onto the annotation type of each item.
///
/// This type is not intended to be used directly.
/// The idiomatic usage would be to import `RenderAdaptorExt` and use method chaining.
/// ```
pub struct RenderAdaptor<'s, A, I, F>
where
    I: Iterator<Item = RenderEvent<'s, A>>,
{
    iterator: I,
    closure: F,
}
impl<'s, A, I, F, B> RenderAdaptor<'s, A, I, F>
where
    I: Iterator<Item = RenderEvent<'s, A>>,
    F: FnMut(A) -> B,
{
    pub fn new<'c>(iterator: I, closure: F) -> Self
    where
        Self: 'c,
        F: 'c,
    {
        Self { iterator, closure }
    }
}
impl<'s, A, I, F, B> Iterator for RenderAdaptor<'s, A, I, F>
where
    I: Iterator<Item = RenderEvent<'s, A>>,
    F: FnMut(A) -> B,
{
    type Item = RenderEvent<'s, B>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|event| match event {
            RenderEvent::PushAnnotation(annotation) => {
                RenderEvent::PushAnnotation((self.closure)(annotation))
            }
            // Yes, we do need to deconstruct + reconstruct every variant
            // because `B` might have a different size from `A`.
            // Most fields here are `Copy` anyways so it isn't too bad.
            RenderEvent::Text(span, s) => RenderEvent::Text(span, s),
            RenderEvent::Linebreak => RenderEvent::Linebreak,
            RenderEvent::Padding(num) => RenderEvent::Padding(num),
            RenderEvent::PopAnnotation => RenderEvent::PopAnnotation,
            RenderEvent::Error(error) => RenderEvent::Error(error),
        })
    }
}

/// Extension trait for iterator chaining of `.map_annotation(f)` and `.strip_annotation()`.
///
/// This trait is blanket-implemented for compatible iterators.
pub trait RenderAdaptorExt<'s, A>
where
    Self: Iterator<Item = RenderEvent<'s, A>> + Sized,
{
    /// Given a closure, map annotations from self, leaving all other events intact.
    fn map_annotation<'c, B, F>(self, closure: F) -> RenderAdaptor<'s, A, Self, F>
    where
        Self: 'c,
        F: FnMut(A) -> B + 'c,
    {
        RenderAdaptor::new(self, closure)
    }
    fn strip_annotation(self) -> RenderAdaptor<'s, A, Self, Box<dyn FnMut(A)>> {
        let strip: Box<dyn FnMut(A)> = Box::new(|_| ());
        RenderAdaptor::new(self, strip)
    }
}
impl<'s, A, I> RenderAdaptorExt<'s, A> for I where I: Iterator<Item = RenderEvent<'s, A>> {}

/// An extremely trivial trait to determine whether a type is Unit `()`, or any form of reference to it.
/// This is used so that plaintext renderers can ignore its annotation.
///
/// **Do not implement this trait.** Use `.strip_annotation()` if you want to replace annotations with an ignorable unit payload.
pub trait IsTrivial {}
impl IsTrivial for () {}
impl<T: IsTrivial> IsTrivial for &T {}
impl<T: IsTrivial> IsTrivial for &mut T {}

/// Trait describing renderers that receive render events to stream or store text of the formatted document.
pub trait Renderer<A> {
    /// The type signalling a render error.
    type Error;

    /// Receives a render event.
    fn receive<'s>(&mut self, event: RenderEvent<'s, A>) -> Result<(), Self::Error>;
    /// Signals the end of the event stream. Defaults to no-op.
    /// This may be used to flush buffers etc.
    fn finish(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Consumes a render event iterator.
    fn consume<'s>(
        &mut self,
        iterator: impl Iterator<Item = RenderEvent<'s, A>>,
    ) -> Result<(), Self::Error> {
        for event in iterator {
            self.receive(event)?;
        }
        self.finish()
    }
}

/// A plaintext renderer.
#[derive(Debug, Clone, Default)]
pub struct PlaintextRenderer<W> {
    pub inner: W,
    pending_padding: usize,
}
impl<W> PlaintextRenderer<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            pending_padding: 0,
        }
    }
    pub fn render<'s, A>(
        iterator: impl Iterator<Item = RenderEvent<'s, A>>,
    ) -> Result<W, <Self as Renderer<A>>::Error>
    where
        Self: Default + Renderer<A>,
        A: IsTrivial,
    {
        let mut renderer = Self::default();
        renderer.consume(iterator)?;
        Ok(renderer.inner)
    }
}
impl PlaintextRenderer<Vec<u8>> {
    pub fn render_to_string<'s, A>(
        iterator: impl Iterator<Item = RenderEvent<'s, A>>,
    ) -> Result<String, <Self as Renderer<A>>::Error>
    where
        A: IsTrivial,
    {
        Ok(String::try_from(Self::render(iterator)?).unwrap())
    }
}
/// The error type of plaintext rendering.
#[derive(Debug, Error)]
pub enum PlaintextRenderError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    LayoutError(#[from] LayoutError),
}
/// Plaintext rendering does not accept annotations, thus requiring a unit payload.
impl<W, A> Renderer<A> for PlaintextRenderer<W>
where
    W: std::io::Write,
    A: IsTrivial,
{
    type Error = PlaintextRenderError;

    fn receive<'s>(&mut self, event: RenderEvent<'s, A>) -> Result<(), Self::Error> {
        match event {
            RenderEvent::Text(_, s) => {
                if self.pending_padding > 0 {
                    self.inner
                        .write_all(" ".repeat(self.pending_padding).as_bytes())?;
                    self.pending_padding = 0;
                }
                self.inner.write_all(s.as_bytes())?;
            }
            RenderEvent::Linebreak => {
                self.pending_padding = 0;
                self.inner.write_all("\n".as_bytes())?;
            }
            RenderEvent::Padding(num) => {
                self.pending_padding += num;
            }
            RenderEvent::PushAnnotation(_) => (),
            RenderEvent::PopAnnotation => (),
            RenderEvent::Error(e) => return Err(PlaintextRenderError::LayoutError(e)),
        }
        Ok(())
    }
    fn finish(&mut self) -> Result<(), Self::Error> {
        Ok(self.inner.flush()?)
    }
}
