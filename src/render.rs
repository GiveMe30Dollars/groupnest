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
    Error(RenderError),
}

/// Errors resulting from layout rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum RenderError {
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
