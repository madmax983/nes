with open("crates/nes-core/src/experimental/spatial_bot.rs", "r") as f:
    content = f.read()

# Add a test case for `new()` to increase coverage
test_case = """
    #[test]
    fn test_spatial_bot_new() {
        let bot = SpatialBot::new();
        assert!(bot.rules.is_empty());
        assert!(bot.active_presses.is_empty());
    }
"""

if "test_spatial_bot_new" not in content:
    content = content.replace("    #[test]\n    fn test_spatial_bot_evaluation() {", test_case + "\n    #[test]\n    fn test_spatial_bot_evaluation() {")

with open("crates/nes-core/src/experimental/spatial_bot.rs", "w") as f:
    f.write(content)
