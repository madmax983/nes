import re

with open('crates/nes-core/src/ppu.rs', 'r') as f:
    content = f.read()

# Define the new struct and helper method
new_helper = """
    fn sprite_pixel_details(&self, sprite_index: usize, x: usize, y: usize) -> Option<(u8, u8, bool)> {
        let base = sprite_index * 4;
        let sprite_y = usize::from(self.oam[base]).wrapping_add(1);
        let sprite_height = if self.ctrl & CTRL_SPRITE_SIZE_8X16 != 0 { 16 } else { 8 };

        if y < sprite_y || y >= sprite_y + sprite_height {
            return None;
        }

        let sprite_x = usize::from(self.oam[base + 3]);
        if x < sprite_x || x >= sprite_x + 8 {
            return None;
        }

        let tile = self.oam[base + 1];
        let attr = self.oam[base + 2];

        let mut local_x = (x - sprite_x) as u8;
        let mut local_y = (y - sprite_y) as u8;
        if attr & 0x40 != 0 {
            local_x = 7 - local_x;
        }
        if attr & 0x80 != 0 {
            local_y = (sprite_height as u8 - 1) - local_y;
        }

        let (pattern_addr, bit) = if sprite_height == 16 {
            let table = u16::from(tile & 1) * 0x1000;
            let tile_top = u16::from(tile & 0xFE);
            let tile_offset = u16::from(local_y / 8);
            let row = u16::from(local_y % 8);
            let addr = table + (tile_top + tile_offset) * 16 + row;
            (addr, 7 - local_x)
        } else {
            let table = if self.ctrl & CTRL_SPRITE_TABLE_ADDR != 0 {
                0x1000
            } else {
                0x0000
            };
            let addr = table + u16::from(tile) * 16 + u16::from(local_y);
            (addr, 7 - local_x)
        };

        let plane0 = self.read_ppu_data(pattern_addr);
        let plane1 = self.read_ppu_data(pattern_addr + 8);
        let low = (plane0 >> bit) & 1;
        let high = (plane1 >> bit) & 1;
        let color = (high << 1) | low;

        if color == 0 {
            return None;
        }

        let palette = attr & 0x03;
        let behind_bg = attr & 0x20 != 0;

        Some((color, palette, behind_bg))
    }

"""

# Search blocks
sprite_palette_index_search = """    fn sprite_palette_index(&self, x: usize, y: usize, bg_opaque: bool) -> Option<u8> {
        if self.mask & MASK_SHOW_SPRITES == 0 {
            return None;
        }
        if x < 8 && self.mask & MASK_SHOW_SPRITE_LEFT == 0 {
            return None;
        }

        let sprite_height = if self.ctrl & CTRL_SPRITE_SIZE_8X16 != 0 {
            16
        } else {
            8
        };

        for sprite in 0..64 {
            let base = sprite * 4;
            let sprite_y = usize::from(self.oam[base]).wrapping_add(1);
            let tile = self.oam[base + 1];
            let attr = self.oam[base + 2];
            let sprite_x = usize::from(self.oam[base + 3]);

            if x < sprite_x || x >= sprite_x + 8 {
                continue;
            }
            if y < sprite_y || y >= sprite_y + sprite_height {
                continue;
            }

            let mut local_x = (x - sprite_x) as u8;
            let mut local_y = (y - sprite_y) as u8;
            if attr & 0x40 != 0 {
                local_x = 7 - local_x;
            }
            if attr & 0x80 != 0 {
                local_y = (sprite_height as u8 - 1) - local_y;
            }

            let (pattern_addr, bit) = if sprite_height == 16 {
                let table = u16::from(tile & 1) * 0x1000;
                let tile_top = u16::from(tile & 0xFE);
                let tile_offset = u16::from(local_y / 8);
                let row = u16::from(local_y % 8);
                let addr = table + (tile_top + tile_offset) * 16 + row;
                (addr, 7 - local_x)
            } else {
                let table = if self.ctrl & CTRL_SPRITE_TABLE_ADDR != 0 {
                    0x1000
                } else {
                    0x0000
                };
                let addr = table + u16::from(tile) * 16 + u16::from(local_y);
                (addr, 7 - local_x)
            };

            let plane0 = self.read_ppu_data(pattern_addr);
            let plane1 = self.read_ppu_data(pattern_addr + 8);
            let low = (plane0 >> bit) & 1;
            let high = (plane1 >> bit) & 1;
            let color = (high << 1) | low;
            if color == 0 {
                continue;
            }

            let behind_bg = attr & 0x20 != 0;
            if behind_bg && bg_opaque {
                continue;
            }

            let palette = attr & 0x03;
            let palette_color =
                self.read_palette(0x3F10 + (u16::from(palette) * 4) + u16::from(color));
            return Some(palette_color);
        }

        None
    }"""

sprite_zero_opaque_search = """    #[must_use]
    fn sprite_zero_opaque_at(&self, x: usize, y: usize) -> bool {
        if self.mask & MASK_SHOW_SPRITES == 0 {
            return false;
        }
        if x < 8 && self.mask & MASK_SHOW_SPRITE_LEFT == 0 {
            return false;
        }

        let sprite_height = if self.ctrl & CTRL_SPRITE_SIZE_8X16 != 0 {
            16
        } else {
            8
        };
        let sprite_y = usize::from(self.oam[0]).wrapping_add(1);
        let tile = self.oam[1];
        let attr = self.oam[2];
        let sprite_x = usize::from(self.oam[3]);

        if x < sprite_x || x >= sprite_x + 8 {
            return false;
        }
        if y < sprite_y || y >= sprite_y + sprite_height {
            return false;
        }

        let mut local_x = (x - sprite_x) as u8;
        let mut local_y = (y - sprite_y) as u8;
        if attr & 0x40 != 0 {
            local_x = 7 - local_x;
        }
        if attr & 0x80 != 0 {
            local_y = (sprite_height as u8 - 1) - local_y;
        }

        let (pattern_addr, bit) = if sprite_height == 16 {
            let table = u16::from(tile & 1) * 0x1000;
            let tile_top = u16::from(tile & 0xFE);
            let tile_offset = u16::from(local_y / 8);
            let row = u16::from(local_y % 8);
            let addr = table + (tile_top + tile_offset) * 16 + row;
            (addr, 7 - local_x)
        } else {
            let table = if self.ctrl & CTRL_SPRITE_TABLE_ADDR != 0 {
                0x1000
            } else {
                0x0000
            };
            let addr = table + u16::from(tile) * 16 + u16::from(local_y);
            (addr, 7 - local_x)
        };

        let plane0 = self.read_ppu_data(pattern_addr);
        let plane1 = self.read_ppu_data(pattern_addr + 8);
        let low = (plane0 >> bit) & 1;
        let high = (plane1 >> bit) & 1;
        ((high << 1) | low) != 0
    }"""

sprite_palette_index_replace = """    fn sprite_palette_index(&self, x: usize, y: usize, bg_opaque: bool) -> Option<u8> {
        if self.mask & MASK_SHOW_SPRITES == 0 {
            return None;
        }
        if x < 8 && self.mask & MASK_SHOW_SPRITE_LEFT == 0 {
            return None;
        }

        for sprite in 0..64 {
            if let Some((color, palette, behind_bg)) = self.sprite_pixel_details(sprite, x, y) {
                if behind_bg && bg_opaque {
                    continue;
                }
                let palette_color =
                    self.read_palette(0x3F10 + (u16::from(palette) * 4) + u16::from(color));
                return Some(palette_color);
            }
        }

        None
    }"""

sprite_zero_opaque_replace = """    #[must_use]
    fn sprite_zero_opaque_at(&self, x: usize, y: usize) -> bool {
        if self.mask & MASK_SHOW_SPRITES == 0 {
            return false;
        }
        if x < 8 && self.mask & MASK_SHOW_SPRITE_LEFT == 0 {
            return false;
        }

        self.sprite_pixel_details(0, x, y).is_some()
    }"""


if sprite_palette_index_search not in content:
    print("Failed to find sprite_palette_index")
    exit(1)
if sprite_zero_opaque_search not in content:
    print("Failed to find sprite_zero_opaque_at")
    exit(1)

content = content.replace(sprite_palette_index_search, new_helper + sprite_palette_index_replace)
content = content.replace(sprite_zero_opaque_search, sprite_zero_opaque_replace)

with open('crates/nes-core/src/ppu.rs', 'w') as f:
    f.write(content)

print("Replaced successfully")
