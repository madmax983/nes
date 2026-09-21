with open("crates/nes-core/src/experimental/spatial_bot.rs", "r") as f:
    content = f.read()

test_code = """
    #[test]
    fn test_spatial_bot_derives() {
        let bot = SpatialBot::default();
        let bot_clone = bot.clone();
        let _ = format!("{:?}", bot_clone);

        let rule = BotRule {
            zone_id: 1,
            button: crate::Button::A,
            duration_frames: 1,
        };
        let rule_clone = rule.clone();
        let _ = format!("{:?}", rule_clone);
    }
"""

content = content.replace(
    "    fn test_spatial_bot_evaluation() {",
    test_code + "\n    #[test]\n    fn test_spatial_bot_evaluation() {"
)

with open("crates/nes-core/src/experimental/spatial_bot.rs", "w") as f:
    f.write(content)
