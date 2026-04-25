import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub type AnyControlObservation = ObservationSnapshot;',
    '/// Matches the observation type used by the underlying profiles.\npub type AnyControlObservation = ObservationSnapshot;'
)

content = content.replace(
    '    Smb(ProfileEnv<SmbProfile>),',
    '    /// Environment tuned for Super Mario Bros using the `SmbProfile`.\n    Smb(ProfileEnv<SmbProfile>),'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    pub fn write_tas_movie(',
    '    /// Serializes the TAS movie constructed during the episode and writes it to disk.\n    pub fn write_tas_movie('
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
