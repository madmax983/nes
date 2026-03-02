use nes_dsl::{RomBuildOptions, build_ines_nrom_rom};
use std::fs;
use std::path::{Path, PathBuf};

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
    let source = r#"
        .org $C000

        .reset reset
        .nmi nmi
        .irq irq

    reset:
        SEI
        CLD
        LDX #$40
        STX $4017
        LDX #$FF
        TXS
        INX
        STX $2000
        STX $2001
        STX $4010
        JSR vblank_wait
        JSR vblank_wait

        LDA #$70
        STA $00       ; sprite y
        LDA #$70
        STA $01       ; sprite x

        LDA $2002     ; reset PPU write latch
        LDA #$3F
        STA $2006
        LDA #$00
        STA $2006

        ; BG palette 0
        LDA #$21
        STA $2007
        LDA #$30
        STA $2007
        LDA #$16
        STA $2007
        LDA #$0F
        STA $2007
        ; BG palette 1
        LDA #$21
        STA $2007
        LDA #$01
        STA $2007
        LDA #$11
        STA $2007
        LDA #$31
        STA $2007
        ; BG palette 2
        LDA #$21
        STA $2007
        LDA #$06
        STA $2007
        LDA #$16
        STA $2007
        LDA #$26
        STA $2007
        ; sprite palette 0
        LDA #$21
        STA $2007
        LDA #$30
        STA $2007
        LDA #$16
        STA $2007
        LDA #$0F
        STA $2007

        LDA #$01
        STA $0201     ; tile
        LDA #$00
        STA $0202     ; attr
        LDA $00
        STA $0200     ; y
        LDA $01
        STA $0203     ; x
        LDA #$02
        STA $4014     ; OAM DMA

        LDA #%10000000
        STA $2000     ; enable NMI
        LDA #%00011110
        STA $2001     ; show BG+sprites

    main_loop:
        JMP main_loop

    vblank_wait:
        BIT $2002
        BPL vblank_wait
        RTS

    nmi:
        PHA
        TXA
        PHA
        TYA
        PHA

        LDA #$01
        STA $4016
        LDA #$00
        STA $4016

        ; Skip A/B/Select/Start bits.
        LDA $4016
        LDA $4016
        LDA $4016
        LDA $4016

        ; Up
        LDA $4016
        AND #$01
        BEQ not_up
        DEC $00
    not_up:

        ; Down
        LDA $4016
        AND #$01
        BEQ not_down
        INC $00
    not_down:

        ; Left
        LDA $4016
        AND #$01
        BEQ not_left
        DEC $01
    not_left:

        ; Right
        LDA $4016
        AND #$01
        BEQ not_right
        INC $01
    not_right:

        LDA $00
        STA $0200
        LDA #$01
        STA $0201
        LDA #$00
        STA $0202
        LDA $01
        STA $0203
        LDA #$02
        STA $4014

        PLA
        TAY
        PLA
        TAX
        PLA
        RTI

    irq:
        RTI
    "#;

    let mut chr = vec![0_u8; 8192];
    // Tile #1: small diamond sprite.
    let tile_1 = [
        0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, // plane 0
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // plane 1
    ];
    chr[16..32].copy_from_slice(&tile_1);

    let options = RomBuildOptions {
        chr_rom: chr,
        ..Default::default()
    };

    build_ines_nrom_rom(source, &options).map_err(|e| e.to_string())
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

#[cfg(test)]
mod tests {
    use super::build_homebrew_rom;

    #[test]
    fn homebrew_rom_has_expected_ines_geometry() {
        let rom = build_homebrew_rom().expect("homebrew rom should build");
        let header = [
            0x4E, 0x45, 0x53, 0x1A, // "NES<EOF>"
            0x01, // 1x 16 KiB PRG
            0x01, // 1x 8 KiB CHR
            0x00, // mapper 0, horizontal mirroring
            0x00, // iNES 1.0 upper flags
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert!(rom.starts_with(&header));
        assert_eq!(rom.len(), 16 + (16 * 1024) + (8 * 1024));
    }
}
