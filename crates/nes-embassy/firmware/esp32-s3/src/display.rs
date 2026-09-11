//! ST7789 display driver: init plus the per-frame presenter.
//!
//! The presenter is generic over the display interface so `board.rs` stays
//! the only file with pin knowledge. Swap `ST7789` for `ILI9341` here (and
//! the geometry in `board.rs`) to retarget the panel.

use embedded_graphics::pixelcolor::{IntoStorage, Rgb565};
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    spi::master::{Config as SpiConfig, Spi},
    time::Rate,
    Blocking,
};
use mipidsi::{
    interface::SpiInterface,
    models::ST7789,
    options::{ColorOrder, Orientation},
    Builder, Display,
};

use crate::board;
use embedded_hal_bus::spi::ExclusiveDevice;
use nes_embassy::FRAME_HEIGHT;
use nes_embassy::FRAME_WIDTH;

/// Concrete display type used by this firmware.
pub type NesDisplay = Display<
    SpiInterface<
        'static,
        ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, embedded_hal_bus::spi::NoDelay>,
        Output<'static>,
    >,
    ST7789,
    Output<'static>,
>;

/// Errors that can escape display setup or presentation.
#[derive(Debug)]
pub enum DisplayError {
    SpiConfig,
    Init,
    Present,
}

/// Initializes the panel from the board pin map and returns the display.
///
/// `spi` is the display's SPI peripheral, moved out of `Peripherals` by the
/// caller; all GPIO numbers come from [`board`].
pub fn init(spi: esp_hal::peripherals::SPI2<'static>) -> Result<NesDisplay, DisplayError> {
    use crate::board::steal_pin;

    let spi = Spi::new(
        spi,
        SpiConfig::default().with_frequency(Rate::from_hz(board::LCD_SPI_HZ)),
    )
    .map_err(|_| DisplayError::SpiConfig)?
    .with_sck(steal_pin(board::LCD_SCK))
    .with_mosi(steal_pin(board::LCD_MOSI));

    let cs = Output::new(
        steal_pin(board::LCD_CS),
        Level::High,
        OutputConfig::default(),
    );
    let dc = Output::new(
        steal_pin(board::LCD_DC),
        Level::Low,
        OutputConfig::default(),
    );
    let rst = Output::new(
        steal_pin(board::LCD_RST),
        Level::High,
        OutputConfig::default(),
    );

    // Backlight on.
    let mut bl = Output::new(
        steal_pin(board::LCD_BL),
        Level::High,
        OutputConfig::default(),
    );
    bl.set_high();

    let spi_dev = ExclusiveDevice::new_no_delay(spi, cs).map_err(|_| DisplayError::SpiConfig)?;

    // mipidsi wants a scratch buffer for batching SPI transfers.
    static mut SPI_BUFFER: [u8; 512] = [0; 512];
    // SAFETY: owned by the display for the rest of the program; nothing else
    // touches it.
    let spi_buffer: &'static mut [u8] = unsafe { &mut *(&raw mut SPI_BUFFER) };
    let di = SpiInterface::new(spi_dev, dc, spi_buffer);

    let mut delay = Delay::new();
    let display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .display_size(board::DISP_W, board::DISP_H)
        .display_offset(0, board::DISP_Y_OFFSET)
        .color_order(ColorOrder::Rgb)
        .orientation(Orientation::default())
        .init(&mut delay)
        .map_err(|_| DisplayError::Init)?;

    Ok(display)
}

/// Presents one RGB565 NES frame (`256 * 240`, row-major).
///
/// Centers the frame on the panel: letterboxes when the panel is bigger,
/// center-crops the 8 px overscan margins when it is narrower (240-wide
/// ST7789).
///
/// With the `qemu` feature the SPI transfer itself is skipped (QEMU models
/// no panel), but the pixel walk still runs so the indexing math stays live.
pub fn present(display: &mut NesDisplay, frame: &[u16]) -> Result<(), DisplayError> {
    debug_assert_eq!(frame.len(), FRAME_WIDTH * FRAME_HEIGHT);

    let (view_w, view_h) = if board::DISP_W >= FRAME_WIDTH as u16 {
        (FRAME_WIDTH as u16, FRAME_HEIGHT as u16)
    } else {
        (board::DISP_W, board::DISP_H.min(FRAME_HEIGHT as u16))
    };
    // Offset into the NES frame when cropping horizontally.
    let crop_x = (FRAME_WIDTH as u16).saturating_sub(view_w) / 2;

    #[cfg(not(feature = "qemu"))]
    {
        let sx = board::DISP_W.saturating_sub(view_w) / 2;
        let sy = board::DISP_H.saturating_sub(view_h) / 2;
        display
            .set_pixels(
                sx,
                sy,
                sx + view_w - 1,
                sy + view_h - 1,
                frame
                    .chunks_exact(FRAME_WIDTH)
                    .take(view_h as usize)
                    .flat_map(move |row| {
                        row.iter()
                            .skip(crop_x as usize)
                            .take(view_w as usize)
                            .map(|&px| rgb565_from_u16(px))
                    }),
            )
            .map_err(|_| DisplayError::Present)?;
    }

    #[cfg(feature = "qemu")]
    {
        let _ = display;
        let mut walked = 0usize;
        for row in frame.chunks_exact(FRAME_WIDTH).take(view_h as usize) {
            for &px in row.iter().skip(crop_x as usize).take(view_w as usize) {
                walked += u32::from(rgb565_from_u16(px).into_storage()) as usize;
            }
        }
        core::hint::black_box(walked);
    }

    Ok(())
}

/// Fills the whole panel with one color (borders, fatal-error screen).
///
/// With the `qemu` feature this is a no-op: there is no panel to clear.
pub fn fill_solid(display: &mut NesDisplay, color: Rgb565) -> Result<(), DisplayError> {
    #[cfg(not(feature = "qemu"))]
    {
        use embedded_graphics::draw_target::DrawTarget;

        display.clear(color).map_err(|_| DisplayError::Present)?;
    }
    #[cfg(feature = "qemu")]
    let _ = (display, color);
    Ok(())
}

/// Reinterprets a native-endian RGB565 word as an `Rgb565` pixel.
const fn rgb565_from_u16(px: u16) -> Rgb565 {
    Rgb565::new(
        ((px >> 11) & 0x1F) as u8,
        ((px >> 5) & 0x3F) as u8,
        (px & 0x1F) as u8,
    )
}
