1. **Optimize `read_line_bounded` Allocations**
   I will execute the following `replace_with_git_merge_diff` calls to update `read_line_bounded` in `crates/nes-mcp/src/main.rs`, `crates/nes-relay/src/main.rs`, `crates/nes-desktop/src/mcp_host.rs`, and `crates/nes-desktop/src/netplay.rs`:

   ```rust
   <<<<<<< SEARCH
               if let Some(idx) = newline_idx {
                   let bytes = &available[..=idx];
                   line.push_str(&String::from_utf8_lossy(bytes));
                   (true, idx + 1)
               } else {
                   let bytes = &available[..check_len];
                   line.push_str(&String::from_utf8_lossy(bytes));
                   (total_read + check_len >= limit, check_len)
               }
   =======
               if let Some(idx) = newline_idx {
                   let bytes = &available[..=idx];
                   if let Ok(s) = std::str::from_utf8(bytes) {
                       line.push_str(s);
                   } else {
                       line.push_str(&String::from_utf8_lossy(bytes));
                   }
                   (true, idx + 1)
               } else {
                   let bytes = &available[..check_len];
                   if let Ok(s) = std::str::from_utf8(bytes) {
                       line.push_str(s);
                   } else {
                       line.push_str(&String::from_utf8_lossy(bytes));
                   }
                   (total_read + check_len >= limit, check_len)
               }
   >>>>>>> REPLACE
   ```

2. **Optimize `Vec` mappings avoiding `.collect::<Vec<_>>()`**
   I will execute the following `replace_with_git_merge_diff` calls to update vectors pre-allocation in `crates/nes-desktop/src/menu.rs`, `crates/nes-desktop/src/manual_state.rs`, and `crates/nes-desktop/src/session.rs`:

   For `crates/nes-desktop/src/menu.rs`:
   ```rust
   <<<<<<< SEARCH
   fn slot_entries(ctor: fn(u8) -> AppAction, prefix: &str, slot_count: u8) -> Vec<DesktopMenuEntry> {
       (1..=slot_count)
           .map(|slot| DesktopMenuEntry::Item(item(ctor(slot), &format!("{prefix} {slot}"))))
           .collect()
   }
   =======
   fn slot_entries(ctor: fn(u8) -> AppAction, prefix: &str, slot_count: u8) -> Vec<DesktopMenuEntry> {
       let mut entries = Vec::with_capacity(usize::from(slot_count));
       for slot in 1..=slot_count {
           entries.push(DesktopMenuEntry::Item(item(ctor(slot), &format!("{prefix} {slot}"))));
       }
       entries
   }
   >>>>>>> REPLACE
   ```

   For `crates/nes-desktop/src/manual_state.rs`:
   ```rust
   <<<<<<< SEARCH
   pub fn slot_paths_for_rom(
       rom_path: &Path,
       rom_hash: &str,
       slots: RangeInclusive<u8>,
   ) -> Vec<PathBuf> {
       slots
           .map(|slot| slot_path_for_rom(rom_path, rom_hash, slot))
           .collect()
   }
   =======
   pub fn slot_paths_for_rom(
       rom_path: &Path,
       rom_hash: &str,
       slots: RangeInclusive<u8>,
   ) -> Vec<PathBuf> {
       let mut paths = Vec::with_capacity(slots.end().saturating_sub(*slots.start()) as usize + 1);
       for slot in slots {
           paths.push(slot_path_for_rom(rom_path, rom_hash, slot));
       }
       paths
   }
   >>>>>>> REPLACE
   ```

   For `crates/nes-desktop/src/session.rs`:
   ```rust
   <<<<<<< SEARCH
   pub(crate) fn load_slot_metadata_for_rom(
       rom_path: &Path,
       rom_hash: &str,
   ) -> Result<Vec<SaveSlotMetadata>, String> {
       slot_paths_for_rom(rom_path, rom_hash, 1..=SAVE_SLOT_COUNT)
           .into_iter()
           .map(|path| read_slot_metadata(&path, rom_hash))
           .collect()
   }
   =======
   pub(crate) fn load_slot_metadata_for_rom(
       rom_path: &Path,
       rom_hash: &str,
   ) -> Result<Vec<SaveSlotMetadata>, String> {
       let paths = slot_paths_for_rom(rom_path, rom_hash, 1..=SAVE_SLOT_COUNT);
       let mut metadata = Vec::with_capacity(paths.len());
       for path in paths {
           metadata.push(read_slot_metadata(&path, rom_hash)?);
       }
       Ok(metadata)
   }
   >>>>>>> REPLACE
   ```

3. **Run all workspace tests**
   I will run `cargo test --workspace --all-features` in a bash session to verify everything passes without regressions.

Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

5. **Submit the PR**
   Using `submit` to create a PR named `bolt-avoid-string-allocs-and-collects` with the details.
