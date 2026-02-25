use pretty_assertions::assert_eq;

#[macros::timed]
fn parse(input: &str) -> Result<u32, std::num::ParseIntError> {
    let value: u32 = input.parse()?;
    if value == 0 {
        return Ok(1);
    }
    return Ok(value);
}

struct Counter(u32);

impl Counter {
    #[macros::timed]
    fn bump(&mut self) -> u32 {
        self.0 += 1;
        return self.0;
    }
}

#[macros::timed]
fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    return items
        .iter()
        .copied()
        .reduce(|left, right| if right > left { right } else { left });
}

#[test]
fn keeps_return_values_and_errors() {
    assert_eq!(parse("7"), Ok(7));
    assert_eq!(parse("0"), Ok(1));
    assert!(parse("x").is_err());
}

#[test]
fn works_on_methods_and_generics() {
    let mut counter = Counter(0);
    assert_eq!(counter.bump(), 1);
    assert_eq!(counter.bump(), 2);
    assert_eq!(largest(&[3, 9, 2]), Some(9));
}
