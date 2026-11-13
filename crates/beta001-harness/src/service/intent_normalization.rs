//! Immutable Unicode 17 intent recipe; changing it requires a new profile.

use unicode_normalization::UnicodeNormalization;

pub(super) const PROFILE: &str = "intent-nfkc-u17-v1";

const _: () = assert!(
    unicode_normalization::UNICODE_VERSION.0 == 17
        && unicode_normalization::UNICODE_VERSION.1 == 0
        && unicode_normalization::UNICODE_VERSION.2 == 0
);

fn retained(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
        || !matches!(c, '\u{0000}'..='\u{001f}' | '\u{007f}'..='\u{009f}')
}

pub(super) fn normalize_intent(raw: &str) -> String {
    raw.chars().filter(|&c| retained(c)).nfkc().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_filter_matches_original_for_every_scalar() {
        for c in (0..=0x10ffff).filter_map(char::from_u32) {
            assert_eq!(
                retained(c),
                !c.is_control() || matches!(c, '\t' | '\n' | '\r')
            );
        }
    }

    #[test]
    fn preserves_unicode17_ordering_filtering_and_unassigned_scalars() {
        for (raw, expected) in [
            ("", ""),
            ("\0\u{7f}\u{9f}", ""),
            (" \t\n\r ", " \t\n\r "),
            (
                "Diagnostic \u{212a} report \0generation",
                "Diagnostic K report generation",
            ),
            ("\u{209d}\u{1b123}", "\u{209d}\u{1b123}"),
            ("b\u{301}\u{5c8}", "b\u{301}\u{5c8}"),
            ("a\u{5c8}\u{301}", "a\u{5c8}\u{301}"),
            ("a\u{301}", "\u{e1}"),
            ("a\u{301}\u{323}", "\u{1ea1}\u{301}"),
            ("\u{1100}\u{1161}", "\u{ac00}"),
            ("\u{ffff}\u{10ffff}", "\u{ffff}\u{10ffff}"),
        ] {
            assert_eq!(normalize_intent(raw), expected);
        }
        let raw = format!("a{}", "\u{301}".repeat(256));
        assert_eq!(
            normalize_intent(&raw),
            format!("\u{e1}{}", "\u{301}".repeat(255))
        );
    }

    // This test exercises raw normalization, not the service's control filter.
    // No official corpus is bundled and an unavailable corpus is never a pass.
    #[test]
    #[ignore = "Blocked: requires authorized owner-provided Unicode 17 conformance and age data"]
    fn owner_provided_unicode17_conformance_oracle() {
        use sha2::{Digest, Sha256};

        fn supplied(name: &str) -> String {
            let path = std::env::var(format!("SOVEREIGN_U17_{name}_PATH")).expect("corpus path");
            let expected = std::env::var(format!("SOVEREIGN_U17_{name}_SHA256"))
                .expect("approved corpus SHA-256");
            let bytes = std::fs::read(path).expect("read owner-provided corpus");
            assert_eq!(hex::encode(Sha256::digest(&bytes)), expected);
            String::from_utf8(bytes).expect("UTF-8 corpus")
        }
        let corpus = supplied("NORMALIZATION_TEST");
        let ages = supplied("DERIVED_AGE");
        check_oracle(&corpus, &ages);
    }

    fn check_oracle(corpus: &str, ages: &str) {
        use std::collections::HashSet;

        fn scalars(column: &str) -> String {
            column
                .split_whitespace()
                .map(|s| char::from_u32(u32::from_str_radix(s, 16).unwrap()).unwrap())
                .collect()
        }
        assert!(corpus.starts_with("# NormalizationTest-17.0.0.txt"));
        assert!(ages.starts_with("# DerivedAge-17.0.0.txt"));
        let mut part1 = false;
        let mut listed = HashSet::new();
        let mut rows = 0;
        for line in corpus.lines() {
            if line.starts_with('@') {
                part1 = line.starts_with("@Part1");
                continue;
            }
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                continue;
            }
            let c: Vec<_> = data.split(';').take(5).map(scalars).collect();
            assert_eq!(c.len(), 5);
            if part1 {
                listed.extend(c[0].chars());
            }
            for (i, s) in c.iter().enumerate() {
                assert_eq!(s.nfc().collect::<String>(), c[if i < 3 { 1 } else { 3 }]);
                assert_eq!(s.nfd().collect::<String>(), c[if i < 3 { 2 } else { 4 }]);
                assert_eq!(s.nfkc().collect::<String>(), c[3]);
                assert_eq!(s.nfkd().collect::<String>(), c[4]);
            }
            rows += 1;
        }
        assert!(rows > 0);
        let mut checked = 0;
        for line in ages.lines() {
            let data = line.split('#').next().unwrap().trim();
            if data.is_empty() {
                continue;
            }
            let (range, age) = data.split_once(';').expect("age row");
            let version: Vec<u32> = age.trim().split('.').map(|x| x.parse().unwrap()).collect();
            assert!(version.as_slice() <= [17, 0].as_slice());
            let (lo, hi) = range
                .trim()
                .split_once("..")
                .unwrap_or((range.trim(), range.trim()));
            for c in (u32::from_str_radix(lo, 16).unwrap()..=u32::from_str_radix(hi, 16).unwrap())
                .filter_map(char::from_u32)
                .filter(|c| !listed.contains(c))
            {
                let s = c.to_string();
                assert_eq!(s.nfc().collect::<String>(), s);
                assert_eq!(s.nfd().collect::<String>(), s);
                assert_eq!(s.nfkc().collect::<String>(), s);
                assert_eq!(s.nfkd().collect::<String>(), s);
                checked += 1;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn synthetic_oracle_harness_rejects_wrong_results() {
        // Hand-authored ASCII/Kelvin vectors, not a copied or official corpus.
        let corpus = "# NormalizationTest-17.0.0.txt\n# synthetic harness self-test\n@Part1\n0041;0041;0041;0041;0041;\n@Part2\n212A;004B;004B;004B;004B;\n";
        let ages = "# DerivedAge-17.0.0.txt\n# synthetic harness self-test\n0041..0042 ; 1.1\n0043 ; 1.1\n";
        check_oracle(corpus, ages);
        let wrong = corpus.replace("212A;004B;004B;004B;004B", "212A;004B;004B;006B;004B");
        assert!(std::panic::catch_unwind(|| check_oracle(&wrong, ages)).is_err());
        assert!(std::panic::catch_unwind(|| check_oracle(
            &corpus.replace("17.0.0", "18.0.0"),
            ages
        ))
        .is_err());
    }
}
