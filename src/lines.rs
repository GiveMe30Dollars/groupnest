//! This module contains the functions used to identify, validate and split strings containing vertical whitespace.
//!
//! # Comparison with `std::str::lines`
//!
//! `std::str::lines` identifies the following as newline sequences:
//! - `'\n'`: NL, Newline
//! - `'\r'`: CR, Carriage Return
//! - `"\r\n"`: CRLF sequence, ie. Windows newline.
//!
//! In addition to the above, this module recognises the following as newline sequences:
//! - `'\u{000B}'`: VT, Line Tabulation
//! - `'\u{000c}'`: FF, Form Feed
//! - `'\u{0085}'`: NEL, Next Line
//! - `'\u{2028}'`: LS, Line Separator
//! - `'\u{2029}'`: PS, Paragraph Separator

use std::borrow::Cow;

/// Given a UTF-8 string and a byte index, report whether this character is a newline character,
/// and if yes, return the byte length of the newline token.
///
/// # Panics
///
/// Panics if `index` exceeds byte length of `text`.
pub fn linebreak_len(text: &str, index: usize) -> Option<usize> {
    const NEWLINE_CHARS: [char; 7] = [
        '\r',       // CR, Carriage Return ("\r\n" is considered one linebreak, handled separately)
        '\n',       // NL, Newline
        '\u{000B}', // VT, Line Tabulation
        '\u{000c}', // FF, Form Feed
        '\u{0085}', // NEL, Next Line
        '\u{2028}', // LS, Line Separator
        '\u{2029}', // PS, Paragraph Separator
    ];
    let rest = &text[index..];
    if rest.starts_with("\r\n") {
        Some(2)
    } else if NEWLINE_CHARS.iter().any(|ch| rest.starts_with(*ch)) {
        Some(rest.chars().next().unwrap().len_utf8())
    } else {
        None
    }
}

/// Returns (offset, length) of the next linebreak sequence, if any.
pub fn next_linebreak(text: &str, index: usize) -> Option<(usize, usize)> {
    for offset in index..text.len() {
        if let Some(span) = linebreak_len(text, offset) {
            return Some((offset, span));
        }
    }
    None
}

/// An iterator that segmentates strings based on UTF-8 aware vertical linespaces.
/// Unline the iterator produced by `std::str::lines`, this iterator is URF-8 aware, and identifies the following as newlines:
/// - `'\n'`: NL, Newline
/// - `'\r'`: CR, Carriage Return
/// - `"\r\n"`: CRLF sequence, ie. Windows newline.
/// - `'\u{000B}'`: VT, Line Tabulation
/// - `'\u{000c}'`: FF, Form Feed
/// - `'\u{0085}'`: NEL, Next Line
/// - `'\u{2028}'`: LS, Line Separator
/// - `'\u{2029}'`: PS, Paragraph Separator
///
/// This iterator is guaranteed to never return strings containing newline characters.
/// If the string contains trailing newlines, the final line will be the empty string.
#[derive(Debug, Clone)]
pub struct Lines<'a> {
    text: &'a str,
    /// A value greater than the length of the string indicates iterator exhaustion.
    position: usize,
}
impl<'a> Lines<'a> {
    pub fn new(text: &'a str) -> Self {
        Lines { text, position: 0 }
    }
}
impl<'a> From<&'a str> for Lines<'a> {
    fn from(text: &'a str) -> Self {
        Self::new(text)
    }
}
impl<'a> Iterator for Lines<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        // Position must be strictly greater than, to handle trailing newlines.
        if self.position > self.text.len() {
            return None;
        }
        let start = self.position;
        if let Some((offset, span)) = next_linebreak(self.text, start) {
            self.position = offset + span;
            Some(&self.text[start..offset])
        } else {
            // Exhausted.
            self.position = self.text.len() + 1;
            Some(&self.text[start..])
        }
    }
}

/// An iterator that segmentates strings based on UTF-8 aware vertical linespaces.
/// This iterator is guaranteed to never return strings containing newline characters.
/// If the string contains trailing newlines, the final line will be the empty string.
///
/// # Comparison with `Lines`
///
/// This iterator returns owned or borrowed strings (typed `Cow<'a, str>`) depending on
/// whether the string used to instantiate it is owned or borrowed.
/// Therefore, it takes ownership of its text upon construction.
pub struct LinesCow<'a> {
    text: Cow<'a, str>,
    /// A value greater than the length of the string indicates iterator exhaustion.
    position: usize,
}
impl<'a> LinesCow<'a> {
    pub fn new(cow: Cow<'a, str>) -> Self {
        LinesCow {
            text: cow,
            position: 0,
        }
    }
}
impl<'a> From<Cow<'a, str>> for LinesCow<'a> {
    fn from(cow: Cow<'a, str>) -> Self {
        Self::new(cow)
    }
}
impl<'a> Iterator for LinesCow<'a> {
    type Item = Cow<'a, str>;
    fn next(&mut self) -> Option<Self::Item> {
        // Position must be strictly greater than, to handle trailing newlines.
        if self.position > self.text.len() {
            return None;
        }
        let start = self.position;
        if let Some((offset, span)) = next_linebreak(&self.text, start) {
            self.position = offset + span;
            Some(match &self.text {
                Cow::Borrowed(slice) => Cow::Borrowed(&slice[start..offset]),
                Cow::Owned(inner) => Cow::Owned(inner[start..offset].to_owned()),
            })
        } else {
            // Exhausted.
            self.position = self.text.len() + 1;
            Some(match &self.text {
                Cow::Borrowed(slice) => Cow::Borrowed(&slice[start..]),
                Cow::Owned(inner) => Cow::Owned(inner[start..].to_owned()),
            })
        }
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use crate::lines::{Lines, LinesCow, linebreak_len};
    use expect_test::expect;

    #[test]
    fn newline_identification() {
        let haystack = "\r\n\n\r\u{000B}";
        let result = haystack
            .char_indices()
            .map(|(by, _)| linebreak_len(haystack, by))
            .collect::<Vec<_>>();
        expect![[r#"
            [
                Some(
                    2,
                ),
                Some(
                    1,
                ),
                Some(
                    1,
                ),
                Some(
                    1,
                ),
                Some(
                    1,
                ),
            ]
        "#]]
        .assert_debug_eq(&result);
    }

    #[test]
    fn segmentate() {
        let haystack = "The quick brown fox\r\njumps over\u{2029}the lazy dog.\n";
        let result = Lines::from(haystack).collect::<Vec<_>>();
        expect![[r#"
            [
                "The quick brown fox",
                "jumps over",
                "the lazy dog.",
                "",
            ]
        "#]]
        .assert_debug_eq(&result);
    }

    #[test]
    fn ownership() {
        // This function should be identical to `segmentate` but its resultant strings are owned.
        let haystack = String::from("The quick brown fox\r\njumps over\u{2029}the lazy dog.\n");
        let result = LinesCow::from(Cow::Owned(haystack)).collect::<Vec<_>>();
        expect![[r#"
            [
                "The quick brown fox",
                "jumps over",
                "the lazy dog.",
                "",
            ]
        "#]]
        .assert_debug_eq(&result);
        assert!(result.iter().all(|elem| matches!(elem, Cow::Owned(_))));
    }
}
