import re

with open("crates/nes-core/src/mapper/mmc3.rs", "r") as f:
    content = f.read()

# Replace queue_chr_1k_update signature and body
content = re.sub(
    r"fn queue_chr_1k_update\(\s*&self,\s*bank: u8,\s*src_offset: usize,\s*window: &\[u8; CHR_WINDOW_BYTES\],\s*old_chr: &\[u8\],\s*planned_offsets: &mut \[Option<usize>\],\s*\) \{\s*let bank_index = usize::from\(self\.normalize_chr_bank\(bank\)\);\s*let dst_start = bank_index \* CHR_BANK_1K;\s*let dst_end = dst_start \+ CHR_BANK_1K;\s*let src_slice = &window\[src_offset\.\.src_offset \+ CHR_BANK_1K\];\s*if src_slice != &old_chr\[dst_start\.\.dst_end\] \{\s*planned_offsets\[bank_index\] = Some\(src_offset\);\s*\}\s*\}",
    r"fn queue_chr_1k_update(\n        &self,\n        bank: u8,\n        src_offset: usize,\n        window: &[u8; CHR_WINDOW_BYTES],\n        planned_offsets: &mut [Option<usize>],\n    ) {\n        let bank_index = usize::from(self.normalize_chr_bank(bank));\n        let dst_start = bank_index * CHR_BANK_1K;\n        let dst_end = dst_start + CHR_BANK_1K;\n        let src_slice = &window[src_offset..src_offset + CHR_BANK_1K];\n        if src_slice != &self.chr_data[dst_start..dst_end] {\n            planned_offsets[bank_index] = Some(src_offset);\n        }\n    }",
    content
)

# Replace queue_chr_2k_update signature and body
content = re.sub(
    r"fn queue_chr_2k_update\(\s*&self,\s*bank: u8,\s*src_offset: usize,\s*window: &\[u8; CHR_WINDOW_BYTES\],\s*old_chr: &\[u8\],\s*planned_offsets: &mut \[Option<usize>\],\s*\) \{\s*let even = bank & !1;\s*self\.queue_chr_1k_update\(even, src_offset, window, old_chr, planned_offsets\);\s*self\.queue_chr_1k_update\(\s*even\.wrapping_add\(1\),\s*src_offset \+ CHR_BANK_1K,\s*window,\s*old_chr,\s*planned_offsets,\s*\);\s*\}",
    r"fn queue_chr_2k_update(\n        &self,\n        bank: u8,\n        src_offset: usize,\n        window: &[u8; CHR_WINDOW_BYTES],\n        planned_offsets: &mut [Option<usize>],\n    ) {\n        let even = bank & !1;\n        self.queue_chr_1k_update(even, src_offset, window, planned_offsets);\n        self.queue_chr_1k_update(\n            even.wrapping_add(1),\n            src_offset + CHR_BANK_1K,\n            window,\n            planned_offsets,\n        );\n    }",
    content
)

# Replace sync_chr_ram_from_ppu_window body
content = re.sub(
    r"let old_chr = self\.chr_data\.clone\(\);\s*let mut planned_offsets = vec\!\[None; usize::from\(self\.chr_bank_count_1k\.max\(1\)\)\];",
    r"let mut planned_offsets = vec![None; usize::from(self.chr_bank_count_1k.max(1))];",
    content
)

content = content.replace(",\n                &old_chr", "")

with open("crates/nes-core/src/mapper/mmc3.rs", "w") as f:
    f.write(content)
