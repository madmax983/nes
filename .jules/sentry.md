## 2024-05-24 - Testing the Sprite Extractor API
**Learning:** Testing components that extract PPU graphics requires injecting specific CHR bits into the `PpuSnapshot` rather than full emulator writes.
**Action:** When testing UI or visualization functions like `extract_chr_ram_bmp`, utilize `NesCore::save_state()`, manipulate `snapshot.ppu.chr` explicitly, and then `core.load_state(&snapshot)` to reliably setup complex states without executing thousands of instructions.
