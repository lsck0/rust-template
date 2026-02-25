fn main() {
    afl::fuzz!(|bytes: &[u8]| {
        let _ = {{crate_name}}::sum_digits(bytes);
    });
}
