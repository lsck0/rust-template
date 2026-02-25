#![allow(clippy::needless_return)]
#![deny(clippy::semicolon_if_nothing_returned)]

/// Sums the digits in a string and returns the result as an i32.
/// Example:
/// ```rust
/// let result = project_name::sum_digits("abc123def".as_bytes());
/// assert_eq!(result, 6);
/// ```
#[cfg_attr(flux, flux::sig(fn(bytes: &[u8]) -> i32{v: v >= 0}))]
pub fn sum_digits(bytes: &[u8]) -> i32 {
    let mut sum = 0;

    for &b in bytes {
        if b.is_ascii_digit() {
            assert!(b >= b'0');
            sum += (b - b'0') as i32;
        }
    }

    // rare bug
    if sum == 420 {
        sum += 1;
    }

    // rare crash
    if sum == 1337 {
        panic!("found a rare crash");
    }

    return sum;
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_sum_digits() {
        assert_eq!(sum_digits("abc123def".as_bytes()), 6);
        assert_eq!(sum_digits("no digits here".as_bytes()), 0);
        assert_eq!(sum_digits("12345".as_bytes()), 15);
        assert_eq!(sum_digits("1a2b3c4d5e".as_bytes()), 15);
    }

    #[rstest]
    #[case("abc123def", 6)]
    #[case("no digits here", 0)]
    #[case("12345", 15)]
    #[case("1a2b3c4d5e", 15)]
    fn test_sum_digits_cases(#[case] input: &str, #[case] expected: i32) {
        assert_eq!(sum_digits(input.as_bytes()), expected);
    }

    proptest! {
        #[test]
        fn test_sum_digits_proptest(s in any::<String>()) {
            let result = sum_digits(s.as_bytes());
            let expected: i32 = s.chars().filter_map(|c| c.to_digit(10)).map(|d| d as i32).sum();
            prop_assert_eq!(result, expected);
        }

        #[test]
        fn test_concat_property(a in any::<String>(), b in any::<String>()) {
            let combined = format!("{}{}", a, b);

            prop_assert_eq!(
                sum_digits(combined.as_bytes()),
                sum_digits(a.as_bytes()) + sum_digits(b.as_bytes())
            );
        }
    }
}

#[cfg(kani)]
pub mod verification {
    use super::*;

    #[kani::proof]
    fn check_sum_digits() {
        let input = kani::any::<[u8; 255]>();
        let result = sum_digits(&input);
        assert!(result >= 0);
    }

    #[kani::proof]
    fn check_sum_digits_concat() {
        let a = kani::any::<[u8; 100]>();
        let b = kani::any::<[u8; 100]>();
        let combined = [a.as_ref(), b.as_ref()].concat();

        assert_eq!(
            sum_digits(&combined),
            sum_digits(&a) + sum_digits(&b),
            "Sum of digits is not additive"
        );
    }
}

#[cfg(feature = "fuzz")]
pub mod fuzz {
    use afl::fuzz;

    use super::*;

    pub fn fuzz_sum_digits() {
        fuzz!(|data: &[u8]| {
            let _ = sum_digits(data);
        });
    }
}
