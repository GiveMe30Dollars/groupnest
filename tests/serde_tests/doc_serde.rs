#![cfg(feature = "serde")]

use groupnest::{ArcDoc, ArcDocBuilder, Arena, BoxDoc, RefDoc, RefDocBuilder};
use proptest::{prelude::*, test_runner::RngSeed};
use serde::{Deserialize, de::DeserializeSeed};
use serde_json::{Deserializer, to_string};

use crate::fuzzing::{arbitrary_boxdoc, arbitrary_settings};

macro_rules! payload_to_comparable {
    ($document:expr, $settings:expr) => {
        ($document)
            .to_plaintext_with($settings)
            .map_err(|error: groupnest::renderer::RenderError| error.to_string())
    };
}

macro_rules! serde_equivalent {
    ($original:expr, BoxDoc, $settings:expr) => {
        let serialized = to_string(&($original)).unwrap();
        let restored: BoxDoc<()> =
            BoxDoc::deserialize(&mut Deserializer::from_str(&serialized)).unwrap();
        assert_eq!(
            payload_to_comparable!($original, ($settings).clone()),
            payload_to_comparable!(restored, ($settings).clone()),
        )
    };
    ($original:expr, RefDoc, $settings:expr) => {
        let serialized = to_string(&($original)).unwrap();
        let arena = Arena::new();
        let builder: RefDocBuilder<'_, '_, ()> = RefDocBuilder::new(&arena);
        let restored: RefDoc<'_, '_, ()> = builder
            .deserialize(&mut Deserializer::from_str(&serialized))
            .unwrap();
        assert_eq!(
            payload_to_comparable!($original, ($settings).clone()),
            payload_to_comparable!(restored, ($settings).clone()),
        )
    };
    ($original:expr, ArcDoc, $settings:expr) => {
        let serialized = to_string(&($original)).unwrap();
        let builder: ArcDocBuilder<()> = ArcDocBuilder::new();
        let restored: ArcDoc<()> = builder
            .deserialize(&mut Deserializer::from_str(&serialized))
            .unwrap();
        assert_eq!(
            payload_to_comparable!($original, ($settings).clone()),
            payload_to_comparable!(restored, ($settings).clone()),
        )
    };
}

const PROPTEST_CASES: u32 = 500;
const RNG_SEED: RngSeed = RngSeed::Fixed(PROPTEST_CASES as u64);
proptest! {
    #![proptest_config(ProptestConfig {
        cases: PROPTEST_CASES,
        rng_seed : RNG_SEED,
        ..ProptestConfig::default()
    })]
    /// Stress test for arbitrary serialization/deserialization of well-formed documents.
    #[test]
    fn test_arbitrary(doc in arbitrary_boxdoc(), settings in arbitrary_settings()) {
        serde_equivalent!(doc, BoxDoc, settings);
        serde_equivalent!(doc, RefDoc, settings);
        serde_equivalent!(doc, ArcDoc, settings);
    }
}
