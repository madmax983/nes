//! Experimental tools for visualizing the CPU memory map.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
pub struct MemoryVisualizer;

#[cfg(feature = "nova")]
impl MemoryVisualizer {
    pub fn dump_memory_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let width = 256;
        let height = 256;
        let mut rgba = vec![0u8; width * height * 4];

        for addr in 0..=0xFFFF {
            let addr = addr as u16;
            let val = core.read_memory(addr);

            let x = addr % 256;
            let y = addr / 256;
            let idx = ((y as usize) * width + (x as usize)) * 4;

            let (r, g, b) = match addr {
                0x0000..=0x1FFF => (0, val, 0),     // RAM: Green
                0x2000..=0x3FFF => (0, 0, val),     // PPU: Blue
                0x4000..=0x401F => (val, val, 0),   // APU/IO: Yellow
                0x4020..=0x5FFF => (val, 0, val),   // Cartridge Expansion: Purple
                0x6000..=0x7FFF => (0, val, val),   // PRG RAM: Cyan
                0x8000..=0xFFFF => (val, val, val), // PRG ROM: Grayscale
            };

            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = 255;
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn can_dump_memory_bmp() {
        let core = NesCore::new();
        let bmp_data = MemoryVisualizer::dump_memory_bmp(&core).unwrap();
        // Check BMP header magic
        assert_eq!(&bmp_data[0..2], b"BM");
    }
}
