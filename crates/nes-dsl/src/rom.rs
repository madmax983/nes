use crate::{AssembledProgram, DslError, Mirroring, RomBuildOptions};
use crate::assembler::assemble;

pub(crate) const PRG_BANK_BYTES: usize = 16 * 1024;
pub(crate) const CHR_BANK_BYTES: usize = 8 * 1024;
const VEC_NMI: u16 = 0xFFFA;
pub(crate) const VEC_RESET: u16 = 0xFFFC;
const VEC_IRQ: u16 = 0xFFFE;

/// Builds an iNES mapper-0 ROM from DSL source.
///
/// # Errors
///
/// Returns [`DslError`] if assembly fails or ROM layout is invalid.
pub fn build_ines_nrom_rom(source: &str, options: &RomBuildOptions) -> Result<Vec<u8>, DslError> {
    let program = assemble(source)?;
    emit_ines_nrom_rom(&program, options)
}

/// Emits an iNES mapper-0 ROM from an assembled program.
///
/// # Errors
///
/// Returns [`DslError`] when output cannot fit NROM layout constraints.
pub fn emit_ines_nrom_rom(
    program: &AssembledProgram,
    options: &RomBuildOptions,
) -> Result<Vec<u8>, DslError> {
    let uses_lower_bank = program.bytes.keys().any(|addr| *addr < 0xC000);
    let prg_banks = if uses_lower_bank { 2_usize } else { 1_usize };
    let prg_len = prg_banks * PRG_BANK_BYTES;

    let mut prg = vec![0_u8; prg_len];
    let mut prg_written = vec![false; prg_len];

    for (addr, byte) in &program.bytes {
        insert_mapped_byte(
            &mut prg,
            &mut prg_written,
            prg_banks,
            *addr,
            *byte,
            "program bytes",
        )?;
    }

    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_NMI,
        program.nmi_vector,
        "nmi",
    )?;
    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_RESET,
        program.reset_vector,
        "reset",
    )?;
    write_vector(
        &mut prg,
        &mut prg_written,
        prg_banks,
        VEC_IRQ,
        program.irq_vector,
        "irq",
    )?;

    let chr_banks = if options.chr_rom.is_empty() {
        0_usize
    } else {
        options.chr_rom.len().div_ceil(CHR_BANK_BYTES)
    };
    if chr_banks > usize::from(u8::MAX) {
        return Err(DslError::InvalidRomLayout(format!(
            "CHR bank count {} exceeds iNES header limit",
            chr_banks
        )));
    }

    let mut out = Vec::with_capacity(16 + prg.len() + options.chr_rom.len());
    out.extend_from_slice(b"NES\x1A");
    out.push(prg_banks as u8);
    out.push(chr_banks as u8);

    let mut flags6 = 0_u8;
    if options.mirroring == Mirroring::Vertical {
        flags6 |= 0x01;
    }
    out.push(flags6);
    out.push(0_u8);
    out.extend_from_slice(&[0_u8; 8]);
    out.extend_from_slice(&prg);

    if !options.chr_rom.is_empty() {
        out.extend_from_slice(&options.chr_rom);
    }

    Ok(out)
}

fn write_vector(
    prg: &mut [u8],
    prg_written: &mut [bool],
    prg_banks: usize,
    addr: u16,
    value: u16,
    name: &str,
) -> Result<(), DslError> {
    let [lo, hi] = value.to_le_bytes();
    insert_mapped_byte(prg, prg_written, prg_banks, addr, lo, name)?;
    insert_mapped_byte(prg, prg_written, prg_banks, addr + 1, hi, name)?;
    Ok(())
}

fn insert_mapped_byte(
    prg: &mut [u8],
    prg_written: &mut [bool],
    prg_banks: usize,
    addr: u16,
    byte: u8,
    name: &str,
) -> Result<(), DslError> {
    let Some(offset) = cpu_addr_to_prg_offset(prg_banks, addr) else {
        return Err(DslError::InvalidRomLayout(format!(
            "address ${addr:04X} from {name} does not map to PRG ROM"
        )));
    };
    if prg_written[offset] && prg[offset] != byte {
        return Err(DslError::InvalidRomLayout(format!(
            "address ${addr:04X} written multiple times with different values"
        )));
    }
    prg[offset] = byte;
    prg_written[offset] = true;
    Ok(())
}

fn cpu_addr_to_prg_offset(prg_banks: usize, addr: u16) -> Option<usize> {
    if prg_banks == 1 {
        if (0xC000..=0xFFFF).contains(&addr) {
            return Some((addr - 0xC000) as usize);
        }
    } else if prg_banks == 2 {
        if (0x8000..=0xFFFF).contains(&addr) {
            return Some((addr - 0x8000) as usize);
        }
    }
    None
}
