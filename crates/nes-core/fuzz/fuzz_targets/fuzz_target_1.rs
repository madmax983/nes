#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Generate valid iNES headers so that load_ines_rom() actually parses it and sets up mappers, etc.
    let mut rom = Vec::new();
    rom.extend_from_slice(b"NES\x1A");

    if data.len() < 4 {
        return;
    }

    // To ensure the ROM bytes cover PRG length.
    let prg_banks = std::cmp::max(1, data[0]) % 4; // limit memory so tests don't immediately fail.
    let chr_banks = data[1] % 4;

    rom.push(prg_banks);
    rom.push(chr_banks);
    rom.push(data[2]);
    rom.push(data[3]);

    rom.extend_from_slice(&[0; 8]); // padding

    let needed = (prg_banks as usize) * 16384 + (chr_banks as usize) * 8192;
    if data.len() - 4 < needed {
         let mut payload = data[4..].to_vec();
         payload.resize(needed, 0);
         rom.extend_from_slice(&payload);
    } else {
        rom.extend_from_slice(&data[4..]); // rest of arbitrary payload
    }

    let mut core = nes_core::NesCore::new();
    if let Ok(_) = core.load_ines_rom(&rom) {
        for _ in 0..100 {
            let _ = core.execute(nes_core::Command::StepFrame);
        }
    }
});
