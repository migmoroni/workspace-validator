/// Returns the sum exercised by the intentional failure fixture.
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn intentional_failure_exposes_diagnostics() {
        assert_eq!(
            add(2, 2),
            5,
            "intentional workspace-validator fixture failure"
        );
    }
}
