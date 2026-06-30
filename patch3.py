import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for RtaSessionState
content = content.replace(
    'pub enum RtaSessionState {',
    '/// Current state of the RTA session.\npub enum RtaSessionState {'
)
content = content.replace(
    '    Idle,\n',
    '    /// Waiting for start condition.\n    Idle,\n'
)
content = content.replace(
    '    Armed,\n',
    '    /// Ready and armed to start.\n    Armed,\n'
)
content = content.replace(
    '    Running,\n',
    '    /// Actively recording a run.\n    Running,\n'
)
content = content.replace(
    '    Finished,\n',
    '    /// Run is completed.\n    Finished,\n'
)
content = content.replace(
    '    InvalidPractice,\n',
    '    /// Run was invalidated.\n    InvalidPractice,\n'
)

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
