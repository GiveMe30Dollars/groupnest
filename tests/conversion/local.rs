use groupnest::{
    ArcDoc, ArcDocBuilder, Arena, BoxDoc, RefDoc, RefDocBuilder,
    document::Document,
    layout::{LayoutMode, LayoutSettings, LayoutWidthConstraint},
};
use proptest::{prelude::*, test_runner::RngSeed};

use crate::fuzzing::{arbitrary_boxdoc, arbitrary_settings};

macro_rules! make {
    ($seed:expr, RefDoc, $original:ident, $continuation:expr) => {{
        let arena = Arena::new();
        let builder = RefDocBuilder::new(&arena);
        let $original: RefDoc<'static, '_, ()> = ($seed)(|inner| builder.alloc(inner));
        $continuation
    }};
    ($seed:expr, BoxDoc, $original:ident, $continuation:expr) => {{
        let $original: BoxDoc<()> = ($seed)(Into::into);
        $continuation
    }};
    ($seed:expr, ArcDoc, $original:ident, $continuation:expr) => {{
        let builder = ArcDocBuilder::new();
        let $original: ArcDoc<()> = ($seed)(|inner| builder.alloc(inner));
        $continuation
    }};
}
macro_rules! to {
    ($original:ident, RefDoc, $converted:ident, $continuation:expr) => {{
        let arena = Arena::new();
        let builder = RefDocBuilder::new(&arena);
        let $converted: RefDoc<'static, '_, ()> =
            ($original).to_representation(&mut |inner| builder.alloc(inner));
        $continuation
    }};
    ($original:ident, BoxDoc, $converted:ident, $continuation:expr) => {{
        let $converted: BoxDoc<()> = ($original).to_representation(&mut Into::into);
        $continuation
    }};
    ($original:ident, ArcDoc, $converted:ident, $continuation:expr) => {{
        let builder = ArcDocBuilder::new();
        let $converted: ArcDoc<()> =
            ($original).to_representation(&mut |inner| builder.alloc(inner));
        $continuation
    }};
}

macro_rules! payload_to_comparable {
    ($document:ident, $settings:expr) => {
        ($document)
            .to_plaintext_with($settings)
            .map_err(|error: groupnest::renderer::RenderError| error.to_string())
    };
}
macro_rules! compare_raw {
    ($seed:expr, $from:tt, $to:tt, $settings:expr, $original:ident, $converted:ident) => {
        make! {
            $seed, $from, $original,
            to!{
                $original, $to, $converted,
                {
                    assert_eq!(
                        payload_to_comparable!($original, $settings),
                        payload_to_comparable!($converted, $settings),
                    )
                }
            }
        }
    };
}
macro_rules! compare {
    ($seed:expr, $from:tt, $to:tt, $settings:expr) => {
        compare_raw!($seed, $from, $to, $settings, original, converted)
    };
}

/// The main workhouse of this testing file.
/// `seed`: a closure that takes in an `alloc` closure and produces a Document type as prescribed.
/// `settings`: the settings for both layout engines.
macro_rules! compare_all {
    ($seed:expr, $settings:expr) => {
        compare!($seed, RefDoc, BoxDoc, ($settings).clone());
        compare!($seed, RefDoc, ArcDoc, ($settings).clone());
        compare!($seed, BoxDoc, RefDoc, ($settings).clone());
        compare!($seed, BoxDoc, ArcDoc, ($settings).clone());
        compare!($seed, ArcDoc, RefDoc, ($settings).clone());
        compare!($seed, ArcDoc, BoxDoc, ($settings).clone());
    };
}

const DEFAULT_SETTINGS: LayoutSettings = LayoutSettings {
    min_width: 20,
    max_width: 100,
    width_constraint: LayoutWidthConstraint::Relaxed,
    initial_mode: LayoutMode::Flat,
};

#[test]
fn conversion_nil() {
    compare_all!(Document::nil, DEFAULT_SETTINGS);
}
#[test]
fn conversion_text() {
    let text = "Lorem ipsum dolor sit amet";
    compare_all!(
        |alloc| Document::flat_text(text, alloc).unwrap(),
        DEFAULT_SETTINGS
    );
}
#[test]
fn conversion_break() {
    let flat = "but if you";
    let broken = "close\nyour\neyes";
    compare_all!(
        |alloc| Document::breaker(flat, broken, alloc).unwrap(),
        DEFAULT_SETTINGS
    );
}
#[test]
fn conversion_hard_linebreak() {
    compare_all!(Document::hard_linebreak, DEFAULT_SETTINGS);
}
#[test]
fn conversion_sequence() {
    compare_all!(
        |alloc| {
            let children = (1..10)
                .into_iter()
                .map(|inner| Document::flat_text(inner.to_string(), alloc).unwrap())
                .collect::<Vec<_>>();
            Document::sequence_intersperse_with(
                children,
                Document::breaker(", ", ",\n", alloc).unwrap(),
                alloc,
            )
        },
        DEFAULT_SETTINGS
    );
}
#[test]
fn conversion_group() {
    compare_all!(
        |alloc| {
            let children = (1..10)
                .into_iter()
                .map(|inner| Document::flat_text(inner.to_string(), alloc).unwrap())
                .collect::<Vec<_>>();
            let inner = Document::sequence_intersperse_with(
                children,
                Document::breaker(", ", ",\n", alloc).unwrap(),
                alloc,
            );
            Document::grouped_sequence(
                vec![
                    Document::flat_text("[", alloc).unwrap(),
                    inner,
                    Document::breaker("]", ",\n]", alloc).unwrap(),
                ],
                alloc,
            )
        },
        DEFAULT_SETTINGS
    );
}
#[test]
fn conversion_nest() {
    compare_all!(
        |alloc| {
            Document::nest(
                20,
                Document::flat_text("lorem ipsum dolor sit amet", alloc).unwrap(),
                alloc,
            )
        },
        DEFAULT_SETTINGS
    );
}

const PROPTEST_CASES: u32 = 500;
const RNG_SEED: RngSeed = RngSeed::Fixed(PROPTEST_CASES as u64);
proptest! {
    #![proptest_config(ProptestConfig {
        cases: PROPTEST_CASES,
        rng_seed : RNG_SEED,
        ..ProptestConfig::default()
    })]
    /// This can only really test BoxDoc --> RefDoc, ArcDoc conversion.
    /// Fine by me tbh, this is already overkill for testing `to_representation` functionality.
    #[test]
    fn test_arbitrary(doc in arbitrary_boxdoc(), settings in arbitrary_settings()) {
        to!(
            doc, RefDoc, converted,
            {
                assert_eq!(
                    payload_to_comparable!(doc, settings.clone()),
                    payload_to_comparable!(converted, settings.clone()),
                )
            }
        );
        to!(
            doc, ArcDoc, converted,
            {
                assert_eq!(
                    payload_to_comparable!(doc, settings.clone()),
                    payload_to_comparable!(converted, settings.clone()),
                )
            }
        );
    }
}
