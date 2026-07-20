with open("crates/nes-core/src/experimental/spatial_bot.rs", "r") as f:
    content = f.read()

test_case = """
    #[test]
    fn test_spatial_bot_derives() {
        let bot = SpatialBot::default();
        let cloned_bot = bot.clone();
        assert_eq!(format!("{:?}", bot), format!("{:?}", cloned_bot));

        let rule = BotRule {
            zone_id: 1,
            button: Button::A,
            duration_frames: 10,
        };
        let cloned_rule = rule.clone();
        assert_eq!(format!("{:?}", rule), format!("{:?}", cloned_rule));
    }
"""

if "test_spatial_bot_derives" not in content:
    content = content.replace("    #[test]\n    fn test_spatial_bot_new() {", test_case + "\n    #[test]\n    fn test_spatial_bot_new() {")

with open("crates/nes-core/src/experimental/spatial_bot.rs", "w") as f:
    f.write(content)
