use std::{fs, path::Path};

use pretty_assertions::assert_eq;
use proptest::prelude::*;
use rstest::rstest;

const DIGIT_MAX: u64 = 9;
const FUZZ_REGRESSIONS_DIR: &str = "tests/fuzz/sum_digits/regressions";

#[rstest]
#[case::empty(b"", 0)]
#[case::no_digits(b"no digits here", 0)]
#[case::only_digits(b"12345", 15)]
#[case::digits_between_letters(b"abc123def", 6)]
#[case::alternating(b"1a2b3c4d5e", 15)]
#[case::non_ascii_bytes(&[0xff, b'7', 0x80], 7)]
fn sum_digits_matches_known_case(#[case] input: &[u8], #[case] expected: u64) {
    assert_eq!({{crate_name}}::sum_digits(input), expected);
}

proptest! {
    #[test]
    fn sum_digits_matches_char_digit_sum(text in any::<String>()) {
        let expected: u64 = text.chars().filter_map(|character| character.to_digit(10)).map(u64::from).sum();
        prop_assert_eq!({{crate_name}}::sum_digits(text.as_bytes()), expected);
    }

    #[test]
    fn sum_digits_is_additive_over_concatenation(left in any::<Vec<u8>>(), right in any::<Vec<u8>>()) {
        let combined = [left.as_slice(), right.as_slice()].concat();
        let separate_sum = {{crate_name}}::sum_digits(&left) + {{crate_name}}::sum_digits(&right);
        prop_assert_eq!({{crate_name}}::sum_digits(&combined), separate_sum);
    }

    #[test]
    fn sum_digits_is_bounded_by_digit_max_per_byte(input in any::<Vec<u8>>()) {
        prop_assert!({{crate_name}}::sum_digits(&input) <= DIGIT_MAX * input.len() as u64);
    }
}

#[test]
fn sum_digits_survives_fuzz_regressions() {
    let regressions_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FUZZ_REGRESSIONS_DIR);
    let Ok(entries) = fs::read_dir(&regressions_dir) else {
        return;
    };

    for entry in entries {
        let path = entry.expect("readable regressions directory").path();
        let input = fs::read(&path).expect("readable regression case");
        let _ = {{crate_name}}::sum_digits(&input);
    }
}
