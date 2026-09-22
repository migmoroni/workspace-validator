/// Returns the sum used by the passing validation fixture.
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn valid_code_passes() {
        assert_eq!(add(2, 2), 4);
    }
}
