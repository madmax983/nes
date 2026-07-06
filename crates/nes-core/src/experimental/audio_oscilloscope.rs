#[cfg(all(test, feature = "nova"))]
mod tests {
    #[test]
    fn test_fail() {
        assert_eq!(1, 2);
    }
}
