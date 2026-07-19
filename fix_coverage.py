import os
import glob
import re

mapper_dir = "crates/nes-core/src/mapper"
mappers = glob.glob(f"{mapper_dir}/*.rs")

for filepath in mappers:
    with open(filepath, "r") as f:
        content = f.read()

    # Find `pub(crate) fn restore_state(&mut self, state: MapperState)`
    match = re.search(r'pub\(crate\) fn restore_state\(&mut self, state: ([A-Za-z0-9_]+)\) \{', content)
    if not match:
        continue

    state_type = match.group(1)

    # Add doc comment and modify signature
    new_sig = f"""    /// Restores state from a snapshot.
    ///
    /// Performance note: Takes state by reference to avoid heap allocations
    /// when restoring structs with large dynamically allocated fields.
    pub(crate) fn restore_state(&mut self, state: &{state_type}) {{"""

    new_content = content.replace(match.group(0), new_sig)

    # Handle Vecs that were moved
    # Example: self.prg_ram = state.prg_ram; -> self.prg_ram = state.prg_ram.clone();
    # self.wram = state.wram; -> self.wram = state.wram.clone();
    # self.exram = state.exram; -> self.exram = state.exram.clone();
    new_content = re.sub(r'self\.([a-z_]+) = state\.([a-z_]+);\n(\s+)self\.\1\.resize', r'self.\1 = state.\2.clone();\n\3self.\1.resize', new_content)

    # test fix
    new_content = new_content.replace(".restore_state(state)", ".restore_state(&state)")
    new_content = new_content.replace(".restore_state(s)", ".restore_state(&s)")

    with open(filepath, "w") as f:
        f.write(new_content)

print("Updated mappers.")

filepath = "crates/nes-core/src/api.rs"
with open(filepath, "r") as f:
    content = f.read()

content = content.replace("mapper.restore_state(*state)", "mapper.restore_state(state)")
content = content.replace("mapper.restore_state(state.clone())", "mapper.restore_state(state)")

with open(filepath, "w") as f:
    f.write(content)

print("Updated api.rs")
