use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const INES_HEADER: [u8; 16] = [
    0x4E, 0x45, 0x53, 0x1A, // "NES<EOF>"
    0x01, // 1x 16 KiB PRG
    0x01, // 1x 8 KiB CHR
    0x00, // mapper 0, horizontal mirroring
    0x00, // iNES 1.0 upper flags
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
const PRG_BYTES: usize = 16 * 1024;
const CHR_BYTES: usize = 8 * 1024;
const PRG_ORIGIN: u16 = 0xC000;
const NMI_VECTOR_OFFSET: usize = PRG_BYTES - 6;
const RESET_VECTOR_OFFSET: usize = PRG_BYTES - 4;
const IRQ_VECTOR_OFFSET: usize = PRG_BYTES - 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixupKind {
    Abs16,
    Rel8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Fixup {
    at: usize,
    kind: FixupKind,
    label: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Program {
    bytes: Vec<u8>,
    labels: HashMap<&'static str, u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Assembler {
    origin: u16,
    bytes: Vec<u8>,
    labels: HashMap<&'static str, u16>,
    fixups: Vec<Fixup>,
}

impl Assembler {
    fn new(origin: u16) -> Self {
        Self {
            origin,
            bytes: Vec::new(),
            labels: HashMap::new(),
            fixups: Vec::new(),
        }
    }

    fn pc(&self) -> u16 {
        self.origin + self.bytes.len() as u16
    }

    fn label(&mut self, name: &'static str) -> Result<(), String> {
        if self.labels.insert(name, self.pc()).is_some() {
            return Err(format!("duplicate label '{name}'"));
        }
        Ok(())
    }

    fn op(&mut self, opcode: u8) {
        self.bytes.push(opcode);
    }

    fn imm(&mut self, opcode: u8, value: u8) {
        self.bytes.push(opcode);
        self.bytes.push(value);
    }

    fn zp(&mut self, opcode: u8, addr: u8) {
        self.bytes.push(opcode);
        self.bytes.push(addr);
    }

    fn abs(&mut self, opcode: u8, addr: u16) {
        self.bytes.push(opcode);
        self.bytes.extend_from_slice(&addr.to_le_bytes());
    }

    fn abs_label(&mut self, opcode: u8, label: &'static str) {
        self.bytes.push(opcode);
        let at = self.bytes.len();
        self.bytes.extend_from_slice(&0_u16.to_le_bytes());
        self.fixups.push(Fixup {
            at,
            kind: FixupKind::Abs16,
            label,
        });
    }

    fn rel_label(&mut self, opcode: u8, label: &'static str) {
        self.bytes.push(opcode);
        let at = self.bytes.len();
        self.bytes.push(0);
        self.fixups.push(Fixup {
            at,
            kind: FixupKind::Rel8,
            label,
        });
    }

    fn finalize(mut self) -> Result<Program, String> {
        for fixup in &self.fixups {
            let target = *self
                .labels
                .get(fixup.label)
                .ok_or_else(|| format!("unknown label '{}'", fixup.label))?;
            match fixup.kind {
                FixupKind::Abs16 => {
                    let [lo, hi] = target.to_le_bytes();
                    self.bytes[fixup.at] = lo;
                    self.bytes[fixup.at + 1] = hi;
                }
                FixupKind::Rel8 => {
                    let next_pc = self.origin + fixup.at as u16 + 1;
                    let delta = i32::from(target) - i32::from(next_pc);
                    if !(-128..=127).contains(&delta) {
                        return Err(format!(
                            "branch target '{}' out of range: delta={delta}",
                            fixup.label
                        ));
                    }
                    self.bytes[fixup.at] = (delta as i8) as u8;
                }
            }
        }
        Ok(Program {
            bytes: self.bytes,
            labels: self.labels,
        })
    }
}

#[must_use]
pub fn default_homebrew_rom_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("roms")
        .join("homebrew")
        .join("homebrew.nes")
}

pub fn build_homebrew_rom() -> Result<Vec<u8>, String> {
    let program = assemble_homebrew_program()?;
    if program.bytes.len() > PRG_BYTES {
        return Err(format!(
            "homebrew PRG too large: {} bytes > {} bytes",
            program.bytes.len(),
            PRG_BYTES
        ));
    }

    let nmi = *program
        .labels
        .get("nmi")
        .ok_or_else(|| "missing nmi label".to_owned())?;
    let reset = *program
        .labels
        .get("reset")
        .ok_or_else(|| "missing reset label".to_owned())?;
    let irq = *program
        .labels
        .get("irq")
        .ok_or_else(|| "missing irq label".to_owned())?;

    let mut prg = vec![0_u8; PRG_BYTES];
    prg[..program.bytes.len()].copy_from_slice(&program.bytes);
    prg[NMI_VECTOR_OFFSET..NMI_VECTOR_OFFSET + 2].copy_from_slice(&nmi.to_le_bytes());
    prg[RESET_VECTOR_OFFSET..RESET_VECTOR_OFFSET + 2].copy_from_slice(&reset.to_le_bytes());
    prg[IRQ_VECTOR_OFFSET..IRQ_VECTOR_OFFSET + 2].copy_from_slice(&irq.to_le_bytes());

    let mut chr = vec![0_u8; CHR_BYTES];
    // Tile #1: small diamond sprite.
    let tile_1 = [
        0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, // plane 0
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // plane 1
    ];
    chr[16..32].copy_from_slice(&tile_1);

    let mut rom = Vec::with_capacity(INES_HEADER.len() + PRG_BYTES + CHR_BYTES);
    rom.extend_from_slice(&INES_HEADER);
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&chr);
    Ok(rom)
}

pub fn write_homebrew_rom(path: &Path) -> Result<(), String> {
    let rom = build_homebrew_rom()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create homebrew output directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    fs::write(path, rom)
        .map_err(|err| format!("failed to write homebrew ROM '{}': {err}", path.display()))
}

fn assemble_homebrew_program() -> Result<Program, String> {
    let mut asm = Assembler::new(PRG_ORIGIN);

    const PALETTE: [u8; 16] = [
        0x21, 0x30, 0x16, 0x0F, // BG palette 0
        0x21, 0x01, 0x11, 0x31, // BG palette 1
        0x21, 0x06, 0x16, 0x26, // BG palette 2
        0x21, 0x30, 0x16, 0x0F, // sprite palette 0
    ];
    // Game Design Theory: Increased base speed from 1 to 2. A speed of 1 felt too sluggish ("Friction"). Moving faster feels "Juicier" and more responsive.
    const PLAYER_SPEED: u8 = 2;

    asm.label("reset")?;
    asm.op(0x78); // SEI
    asm.op(0xD8); // CLD
    asm.imm(0xA2, 0x40); // LDX #$40
    asm.abs(0x8E, 0x4017); // STX $4017
    asm.imm(0xA2, 0xFF); // LDX #$FF
    asm.op(0x9A); // TXS
    asm.op(0xE8); // INX
    asm.abs(0x8E, 0x2000); // STX $2000
    asm.abs(0x8E, 0x2001); // STX $2001
    asm.abs(0x8E, 0x4010); // STX $4010
    asm.abs_label(0x20, "vblank_wait"); // JSR vblank_wait
    asm.abs_label(0x20, "vblank_wait"); // JSR vblank_wait

    asm.imm(0xA9, 0x70); // LDA #$70
    asm.zp(0x85, 0x00); // STA $00 (sprite y)
    asm.imm(0xA9, 0x70); // LDA #$70
    asm.zp(0x85, 0x01); // STA $01 (sprite x)

    asm.abs(0xAD, 0x2002); // LDA $2002 (reset PPU write latch)
    asm.imm(0xA9, 0x3F); // LDA #$3F
    asm.abs(0x8D, 0x2006); // STA $2006
    asm.imm(0xA9, 0x00); // LDA #$00
    asm.abs(0x8D, 0x2006); // STA $2006
    for value in PALETTE {
        asm.imm(0xA9, value); // LDA #palette byte
        asm.abs(0x8D, 0x2007); // STA $2007
    }

    asm.imm(0xA9, 0x01); // LDA #$01
    asm.abs(0x8D, 0x0201); // STA $0201 (tile)
    asm.imm(0xA9, 0x00); // LDA #$00
    asm.abs(0x8D, 0x0202); // STA $0202 (attr)
    asm.zp(0xA5, 0x00); // LDA $00
    asm.abs(0x8D, 0x0200); // STA $0200 (y)
    asm.zp(0xA5, 0x01); // LDA $01
    asm.abs(0x8D, 0x0203); // STA $0203 (x)
    asm.imm(0xA9, 0x02); // LDA #$02
    asm.abs(0x8D, 0x4014); // STA $4014 (OAM DMA)

    asm.imm(0xA9, 0x80); // LDA #%1000_0000
    asm.abs(0x8D, 0x2000); // STA $2000 (enable NMI)
    asm.imm(0xA9, 0x1E); // LDA #%0001_1110
    asm.abs(0x8D, 0x2001); // STA $2001 (show BG+sprites)

    asm.label("main_loop")?;
    asm.abs_label(0x4C, "main_loop"); // JMP main_loop

    asm.label("vblank_wait")?;
    asm.abs(0x2C, 0x2002); // BIT $2002
    asm.rel_label(0x10, "vblank_wait"); // BPL vblank_wait
    asm.op(0x60); // RTS

    asm.label("nmi")?;
    asm.op(0x48); // PHA
    asm.op(0x8A); // TXA
    asm.op(0x48); // PHA
    asm.op(0x98); // TYA
    asm.op(0x48); // PHA

    asm.imm(0xA9, 0x01); // LDA #$01
    asm.abs(0x8D, 0x4016); // STA $4016
    asm.imm(0xA9, 0x00); // LDA #$00
    asm.abs(0x8D, 0x4016); // STA $4016

    // Skip A/B/Select/Start bits.
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.abs(0xAD, 0x4016); // LDA $4016

    // Up
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.imm(0x29, 0x01); // AND #$01
    asm.rel_label(0xF0, "not_up"); // BEQ not_up
    asm.zp(0xA5, 0x00); // LDA $00
    asm.op(0x38); // SEC
    asm.imm(0xE9, PLAYER_SPEED); // SBC #PLAYER_SPEED
    asm.zp(0x85, 0x00); // STA $00
    asm.label("not_up")?;

    // Down
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.imm(0x29, 0x01); // AND #$01
    asm.rel_label(0xF0, "not_down"); // BEQ not_down
    asm.zp(0xA5, 0x00); // LDA $00
    asm.op(0x18); // CLC
    asm.imm(0x69, PLAYER_SPEED); // ADC #PLAYER_SPEED
    asm.zp(0x85, 0x00); // STA $00
    asm.label("not_down")?;

    // Left
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.imm(0x29, 0x01); // AND #$01
    asm.rel_label(0xF0, "not_left"); // BEQ not_left
    asm.zp(0xA5, 0x01); // LDA $01
    asm.op(0x38); // SEC
    asm.imm(0xE9, PLAYER_SPEED); // SBC #PLAYER_SPEED
    asm.zp(0x85, 0x01); // STA $01
    asm.label("not_left")?;

    // Right
    asm.abs(0xAD, 0x4016); // LDA $4016
    asm.imm(0x29, 0x01); // AND #$01
    asm.rel_label(0xF0, "not_right"); // BEQ not_right
    asm.zp(0xA5, 0x01); // LDA $01
    asm.op(0x18); // CLC
    asm.imm(0x69, PLAYER_SPEED); // ADC #PLAYER_SPEED
    asm.zp(0x85, 0x01); // STA $01
    asm.label("not_right")?;

    asm.zp(0xA5, 0x00); // LDA $00
    asm.abs(0x8D, 0x0200); // STA $0200
    asm.imm(0xA9, 0x01); // LDA #$01
    asm.abs(0x8D, 0x0201); // STA $0201
    asm.imm(0xA9, 0x00); // LDA #$00
    asm.abs(0x8D, 0x0202); // STA $0202
    asm.zp(0xA5, 0x01); // LDA $01
    asm.abs(0x8D, 0x0203); // STA $0203
    asm.imm(0xA9, 0x02); // LDA #$02
    asm.abs(0x8D, 0x4014); // STA $4014

    asm.op(0x68); // PLA
    asm.op(0xA8); // TAY
    asm.op(0x68); // PLA
    asm.op(0xAA); // TAX
    asm.op(0x68); // PLA
    asm.op(0x40); // RTI

    asm.label("irq")?;
    asm.op(0x40); // RTI

    asm.finalize()
}

#[cfg(test)]
mod tests {
    use super::{CHR_BYTES, INES_HEADER, PRG_BYTES, build_homebrew_rom};

    #[test]
    fn homebrew_rom_has_expected_ines_geometry() {
        let rom = build_homebrew_rom().expect("homebrew rom should build");
        assert!(rom.starts_with(&INES_HEADER));
        assert_eq!(rom.len(), INES_HEADER.len() + PRG_BYTES + CHR_BYTES);
    }
}
