import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Make sure not to double add docs
if '/// Trigger when value equals.' not in content:
    content = content.replace(
        'pub enum TriggerOp {',
        '/// The operation used to compare a memory value to a target.\npub enum TriggerOp {'
    )
    content = re.sub(r'(\s+)Equal,', r'\1/// Trigger when value equals.\1Equal,', content)
    content = re.sub(r'(\s+)NotEqual,', r'\1/// Trigger when value does not equal.\1NotEqual,', content)
    content = re.sub(r'(\s+)GreaterThan,', r'\1/// Trigger when value is strictly greater than.\1GreaterThan,', content)
    content = re.sub(r'(\s+)LessThan,', r'\1/// Trigger when value is strictly less than.\1LessThan,', content)

if '/// A specific condition to trigger a split.' not in content:
    content = content.replace(
        'pub struct SplitRule {',
        '/// A specific condition to trigger a split.\npub struct SplitRule {'
    )
    content = re.sub(r'(\s+)pub name: String,', r'\1/// Name of the split.\1pub name: String,', content)
    content = re.sub(r'(\s+)pub trigger: TriggerRule,', r'\1/// Memory trigger condition.\1pub trigger: TriggerRule,', content)


with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
