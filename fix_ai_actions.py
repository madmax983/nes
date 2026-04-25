import re

with open('crates/nes-ai/src/actions.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum ControlAction {',
    '/// Defines the discrete action space used by the model for controlling the game.\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum ControlAction {'
)

content = content.replace(
    '    Noop,\n    Right,\n    RightA,\n    A,\n    RightB,\n    RightAB,',
    '''    /// Do nothing.
    Noop,
    /// Press only the Right D-pad button.
    Right,
    /// Press Right and A simultaneously (e.g. running jump).
    RightA,
    /// Press only the A button (e.g. standing jump).
    A,
    /// Press Right and B simultaneously (e.g. running).
    RightB,
    /// Press Right, A, and B simultaneously (e.g. fast running jump).
    RightAB,'''
)

content = content.replace(
    '    #[must_use]\n    pub const fn action_count() -> usize {',
    '    /// Returns the total number of actions in the space.\n    #[must_use]\n    pub const fn action_count() -> usize {'
)

content = content.replace(
    '    #[must_use]\n    pub fn controller1_bits(self) -> u8 {',
    '    /// Maps the abstract action to the exact 8-bit NES controller bitfield.\n    #[must_use]\n    pub fn controller1_bits(self) -> u8 {'
)

with open('crates/nes-ai/src/actions.rs', 'w') as f:
    f.write(content)
