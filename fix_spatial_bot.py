import sys

filepath = "crates/nes-core/src/experimental/spatial_bot.rs"
with open(filepath, "r") as f:
    content = f.read()

# Fix the placement of doc comments before #[derive] and #[must_use]
content = content.replace("""#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
/// A rule that defines an automated button press in response to a spatial zone event.
///
/// When an event triggers in `zone_id`, the bot will output `button` and hold it for
/// `duration_frames`.
pub struct BotRule {""", """/// A rule that defines an automated button press in response to a spatial zone event.
///
/// When an event triggers in `zone_id`, the bot will output `button` and hold it for
/// `duration_frames`.
#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct BotRule {""")


content = content.replace("""#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
/// An automated bot that evaluates spatial events and generates controller inputs.
///
/// The bot evaluates events from a `ZoneTracker` and applies any configured `BotRule`s
/// to output a sequence of `Command`s.
pub struct SpatialBot {""", """/// An automated bot that evaluates spatial events and generates controller inputs.
///
/// The bot evaluates events from a `ZoneTracker` and applies any configured `BotRule`s
/// to output a sequence of `Command`s.
#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct SpatialBot {""")

content = content.replace("""    #[must_use]
    /// Creates a new `SpatialBot` with no rules or active presses.
    pub fn new() -> Self {""", """    /// Creates a new `SpatialBot` with no rules or active presses.
    #[must_use]
    pub fn new() -> Self {""")


with open(filepath, "w") as f:
    f.write(content)
