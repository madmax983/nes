import sys

filepath = "crates/nes-core/src/experimental/ppu_visualizer.rs"
with open(filepath, "r") as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if line.strip() == "pub struct PpuVisualizer;":
        new_lines.append("/// A utility for extracting PPU state and visualizing it as BMP images.\n")
        new_lines.append("///\n")
        new_lines.append("/// This provides methods for dumping pattern tables and nametables (with scroll viewports)\n")
        new_lines.append("/// to easily view the internal graphical state of the NES.\n")
    new_lines.append(line)

with open(filepath, "w") as f:
    f.writelines(new_lines)
