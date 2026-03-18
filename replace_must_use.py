import re

with open("crates/nes-desktop/src/overlay.rs", "r") as f:
    code = f.read()

code = code.replace(
    '''impl OverlaySlotSummary {
    /// Returns a human-readable suffix for rendering beside slot actions.
    #[must_use]
    pub fn render_suffix(&self, out: &mut String) {''',
    '''impl OverlaySlotSummary {
    /// Returns a human-readable suffix for rendering beside slot actions.
    pub fn render_suffix(&self, out: &mut String) {'''
)

with open("crates/nes-desktop/src/overlay.rs", "w") as f:
    f.write(code)
