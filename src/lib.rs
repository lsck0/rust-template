//! Example library; replace it with the crate's own API.

pub mod build_info {
    pub const VERSION: &str = env!("BUILD_VERSION");
    pub const COMMIT: &str = env!("BUILD_COMMIT");
    pub const COMMIT_TIME: &str = env!("BUILD_COMMIT_TIME");
    pub const PROFILE: &str = env!("BUILD_PROFILE");
    pub const TARGET: &str = env!("BUILD_TARGET");
}

/// ```
/// assert_eq!({{crate_name}}::sum_digits(b"abc123def"), 6);
/// ```
#[must_use]
#[cfg_attr(flux, flux::sig(fn(bytes: &[u8]) -> u64))]
pub fn sum_digits(bytes: &[u8]) -> u64 {
    let mut sum = 0;

    for &byte in bytes {
        if byte.is_ascii_digit() {
            // flux cannot see through is_ascii_digit; this bound lets it prove the subtraction cannot underflow
            assert!(byte >= b'0');
            sum += u64::from(byte - b'0');
        }
    }

    return sum;
}

#[cfg(kani)]
mod verification {
    use super::sum_digits;

    const DIGIT_MAX: u64 = 9;
    const INPUT_SIZE_BYTES: usize = 8;

    // kani::unwind takes only a literal: one more than the 16 bytes of the concatenated input
    #[kani::proof]
    #[kani::unwind(17)]
    fn sum_digits_is_bounded_by_digit_max_per_byte() {
        let input = kani::any::<[u8; INPUT_SIZE_BYTES]>();
        assert!(sum_digits(&input) <= DIGIT_MAX * INPUT_SIZE_BYTES as u64);
    }

    #[kani::proof]
    #[kani::unwind(17)]
    fn sum_digits_is_additive_over_concatenation() {
        let left = kani::any::<[u8; INPUT_SIZE_BYTES]>();
        let right = kani::any::<[u8; INPUT_SIZE_BYTES]>();
        let combined = [left.as_slice(), right.as_slice()].concat();

        assert_eq!(sum_digits(&combined), sum_digits(&left) + sum_digits(&right));
    }
}
