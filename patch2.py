import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for LoadedProfile
content = content.replace(
    'pub struct LoadedProfile {',
    '/// A profile loaded from disk along with its path.\npub struct LoadedProfile {'
)
content = content.replace(
    'pub path: PathBuf,',
    '/// File path to the profile.\n    pub path: PathBuf,'
)
content = content.replace(
    'pub profile: RtaProfile,',
    '/// The profile contents.\n    pub profile: RtaProfile,'
)

# Add docs for ProfileSelectionSource
content = content.replace(
    'pub enum ProfileSelectionSource {',
    '/// Source of the profile selection.\npub enum ProfileSelectionSource {'
)
content = content.replace(
    'AutoByRomHash,',
    '/// Automatically selected by matching ROM hash.\n    AutoByRomHash,'
)
content = content.replace(
    'ManualOverride,',
    '/// Manually overridden by the user.\n    ManualOverride,'
)

# Add docs for ProfileSelection
content = content.replace(
    'pub struct ProfileSelection {',
    '/// The final selected profile and its selection source.\npub struct ProfileSelection {'
)
content = content.replace(
    'pub selected: LoadedProfile,',
    '/// The selected profile.\n    pub selected: LoadedProfile,'
)
content = content.replace(
    'pub source: ProfileSelectionSource,',
    '/// How the profile was selected.\n    pub source: ProfileSelectionSource,'
)


with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
