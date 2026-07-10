use std::fs;
use std::path::{Path, PathBuf};

/// Resolves the canonical file path for the pre-compiled `homebrew.nes` test ROM.
///
/// This resolves relative to the `CARGO_MANIFEST_DIR` of the `nes-test-harness` crate, ensuring
/// that tests run from anywhere in the workspace will correctly locate the file in `roms/homebrew/`.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::default_homebrew_rom_path;
///
/// let path = default_homebrew_rom_path();
/// assert!(path.ends_with("homebrew.nes"));
/// ```
#[must_use]
pub fn default_homebrew_rom_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("roms")
        .join("homebrew")
        .join("homebrew.nes")
}

/// Compiles the minimal test ROM ("homebrew.nes") using the `nes_dsl` assembler.
///
/// This ROM is used across the codebase for integration testing and serves as a reliable,
/// deterministic target that doesn't require downloading copyrighted material.
///
/// ## Examples
///
/// ```
/// use nes_test_harness::build_homebrew_rom;
///
/// let rom_bytes = build_homebrew_rom().unwrap();
/// assert!(rom_bytes.len() > 16);
/// assert_eq!(&rom_bytes[0..4], b"NES\x1A");
/// ```
pub fn build_homebrew_rom() -> Result<Vec<u8>, String> {
    let source = assemble_homebrew_program();

    let options = nes_dsl::RomBuildOptions {
        chr_rom: {
            let mut chr = vec![0_u8; 8192];
            let tile_1 = [
                0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, // plane 0
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // plane 1
            ];
            chr[16..32].copy_from_slice(&tile_1);
            chr
        },
        ..nes_dsl::RomBuildOptions::default()
    };

    nes_dsl::build_ines_nrom_rom(&source, &options).map_err(|e| e.to_string())
}

/// Compiles and writes the minimal test ROM to the specified path on disk.
///
/// This function also creates any missing parent directories required by the target path.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use nes_test_harness::write_homebrew_rom;
///
/// let path = Path::new("./roms/homebrew/homebrew.nes");
/// write_homebrew_rom(path).unwrap();
/// assert!(path.exists());
/// ```
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

fn assemble_homebrew_program() -> String {
    r#"
.bank 0
.org $C000

.const PPUCTRL = $2000
.const PPUMASK = $2001
.const PPUSTATUS = $2002
.const OAMADDR = $2003
.const PPUSCROLL = $2005
.const PPUADDR = $2006
.const PPUDATA = $2007
.const OAMDMA = $4014

.const APU_DMC = $4010
.const APU_FRAME = $4017
.const JOY1 = $4016
.const JOY2 = $4017

.const SPR_Y = $00
.const SPR_X = $01
.const OAM_Y = $0200
.const OAM_TILE = $0201
.const OAM_ATTR = $0202
.const OAM_X = $0203

reset:
    sei
    cld
    ldx #$40
    stx APU_FRAME
    ldx #$FF
    txs
    inx
    stx PPUCTRL
    stx PPUMASK
    stx APU_DMC
    jsr vblank_wait
    jsr vblank_wait

    lda #$70
    sta SPR_Y
    lda #$70
    sta SPR_X

    lda PPUSTATUS
    lda #$3F
    sta PPUADDR
    lda #$00
    sta PPUADDR

    ; BG palette 0
    lda #$21
    sta PPUDATA
    lda #$30
    sta PPUDATA
    lda #$16
    sta PPUDATA
    lda #$0F
    sta PPUDATA

    ; BG palette 1
    lda #$21
    sta PPUDATA
    lda #$01
    sta PPUDATA
    lda #$11
    sta PPUDATA
    lda #$31
    sta PPUDATA

    ; BG palette 2
    lda #$21
    sta PPUDATA
    lda #$06
    sta PPUDATA
    lda #$16
    sta PPUDATA
    lda #$26
    sta PPUDATA

    ; sprite palette 0
    lda #$21
    sta PPUDATA
    lda #$30
    sta PPUDATA
    lda #$16
    sta PPUDATA
    lda #$0F
    sta PPUDATA

    lda #$01
    sta OAM_TILE
    lda #$00
    sta OAM_ATTR
    lda SPR_Y
    sta OAM_Y
    lda SPR_X
    sta OAM_X
    lda #$02
    sta OAMDMA

    lda #%10000000
    sta PPUCTRL
    lda #%00011110
    sta PPUMASK

main_loop:
    jmp main_loop

vblank_wait:
    bit PPUSTATUS
    bpl vblank_wait
    rts

nmi:
    pha
    txa
    pha
    tya
    pha

    lda #$01
    sta JOY1
    lda #$00
    sta JOY1

    ; Skip A/B/Select/Start
    lda JOY1
    lda JOY1
    lda JOY1
    lda JOY1

    ; Up
    lda JOY1
    and #$01
    beq not_up
    dec SPR_Y
not_up:

    ; Down
    lda JOY1
    and #$01
    beq not_down
    inc SPR_Y
not_down:

    ; Left
    lda JOY1
    and #$01
    beq not_left
    dec SPR_X
not_left:

    ; Right
    lda JOY1
    and #$01
    beq not_right
    inc SPR_X
not_right:

    lda SPR_Y
    sta OAM_Y
    lda #$01
    sta OAM_TILE
    lda #$00
    sta OAM_ATTR
    lda SPR_X
    sta OAM_X
    lda #$02
    sta OAMDMA

    pla
    tay
    pla
    tax
    pla
    rti

irq:
    rti

.reset reset
.nmi nmi
.irq irq
"#
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{build_homebrew_rom, default_homebrew_rom_path, write_homebrew_rom};
    use std::fs;

    #[test]
    fn homebrew_rom_has_expected_ines_geometry() {
        let rom = build_homebrew_rom().expect("homebrew rom should build");
        assert!(rom.starts_with(b"NES\x1A"));
        assert_eq!(rom.len(), 16 + 16 * 1024 + 8 * 1024);
    }

    #[test]
    fn default_homebrew_rom_path_ends_with_homebrew_nes() {
        let path = default_homebrew_rom_path();
        assert!(
            path.ends_with("roms/homebrew/homebrew.nes"),
            "path did not end with roms/homebrew/homebrew.nes, got: {:?}",
            path
        );
    }

    #[test]
    fn build_homebrew_rom_includes_custom_chr_rom_tile() {
        let rom = build_homebrew_rom().expect("homebrew rom should build");

        // CHR ROM starts after 16 byte header and 16KB PRG ROM
        let prg_size = 16 * 1024;
        let chr_offset = 16 + prg_size;

        let expected_tile_1 = [
            0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, // plane 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // plane 1
        ];

        let tile_start = chr_offset + 16;
        let actual_tile = &rom[tile_start..tile_start + 16];
        assert_eq!(
            actual_tile, expected_tile_1,
            "CHR ROM did not contain the expected tile data"
        );
    }

    #[test]
    fn write_homebrew_rom_creates_file_and_parent_directories() {
        let temp_dir = std::env::temp_dir().join("nes_test_harness_homebrew_test");
        let rom_path = temp_dir.join("nested").join("folder").join("test.nes");

        // Clean up any previous runs
        let _ = fs::remove_dir_all(&temp_dir);

        write_homebrew_rom(&rom_path).expect("should write successfully");

        let written_bytes = fs::read(&rom_path).expect("file should exist");
        let generated_rom = build_homebrew_rom().unwrap();

        assert_eq!(
            written_bytes, generated_rom,
            "written bytes should match generated ROM bytes"
        );

        // Clean up after test
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
