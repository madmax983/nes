//! NES on the ESP32-S3.
//!
//! The whole emulation core is hardware-neutral ([`nes_embassy::EmuDriver`]);
//! this binary only wires up the board: heap, RTOS timer, display, buttons,
//! I2S audio, and the 60 Hz game loop.
//!
//! Loop per frame:
//! 1. poll the buttons into a controller bitfield,
//! 2. `EmuDriver::step_frame` — inputs in, one frame emulated, RGB565 frame
//!    + mono audio chunk captured into reusable buffers,
//! 3. push the frame to the ST7789, push the audio chunk to I2S,
//! 4. pace to 60 Hz.

#![no_std]
#![no_main]

extern crate alloc;

mod audio;
mod board;
mod display;
mod input;
mod rom;

use alloc::boxed::Box;
#[cfg(not(feature = "qemu"))]
use embassy_executor::Spawner;
#[cfg(not(feature = "qemu"))]
use embassy_time::{Duration, Instant, Timer};
use esp_alloc::heap_allocator;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};
use nes_embassy::EmuDriver;

use embedded_graphics::pixelcolor::{Rgb565, RgbColor};

/// Target frame period: 60 Hz.
#[cfg(not(feature = "qemu"))]
const FRAME_PERIOD: Duration = Duration::from_micros(16_667);
/// Log a frame counter this often so a serial capture (CI/QEMU) can prove
/// the loop is iterating.
const LOG_EVERY_N_FRAMES: u64 = 60;

#[cfg(not(feature = "qemu"))]
#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    heap_allocator!(size: board::HEAP_BYTES);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    // Serial goes to UART0 (see the `esp-println/uart` dependency): on real
    // hardware it is the USB-UART bridge console, in QEMU it is `-serial`.
    esp_println::println!("nes: boot");

    // Optional second heap region in octal PSRAM for large banked games.
    #[cfg(feature = "psram")]
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    // RTOS + Embassy time driver.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    // Display first so fatal errors have somewhere to show up.
    let mut display = match display::init(peripherals.SPI2) {
        Ok(display) => display,
        Err(_) => hang().await,
    };
    let pad = input::Pad::player_one();
    let mut audio = match audio::init(peripherals.I2S0, peripherals.DMA_CH0) {
        Ok(audio) => audio,
        Err(_) => hang().await,
    };
    let mut dma_buf = match esp_hal::dma_tx_buffer!(audio::DMA_BYTES) {
        Ok(buf) => buf,
        Err(_) => hang().await,
    };

    // The driver (~78 KiB struct + ~100 KiB of mapper/PPU heap for NROM)
    // lives on the heap, not the task stack.
    let mut driver = Box::new(EmuDriver::new());
    if driver.load_rom(rom::ROM).is_err() {
        esp_println::println!("nes: rom load failed");
        let _ = display::fill_solid(&mut display, Rgb565::RED);
        hang().await;
    }
    esp_println::println!("nes: rom loaded, {} bytes", rom::ROM.len());
    // Clear any letterbox borders once; the frame presenter only writes the
    // centered window.
    let _ = display::fill_solid(&mut display, Rgb565::BLACK);
    esp_println::println!("nes: entering frame loop");

    let mut frame: u64 = 0;
    loop {
        let frame_start = Instant::now();

        driver.set_controllers(pad.poll(), 0);
        if driver.step_frame().is_err() {
            esp_println::println!("nes: step_frame failed at frame {frame}");
            let _ = display::fill_solid(&mut display, Rgb565::RED);
            hang().await;
        }
        if display::present(&mut display, driver.frame_rgb565()).is_err() {
            esp_println::println!("nes: present failed at frame {frame}");
            hang().await;
        }
        dma_buf = match audio.push_chunk(driver.audio_chunk(), dma_buf).await {
            Ok(buf) => buf,
            Err(e) => {
                esp_println::println!("nes: audio failed at frame {frame}: {e:?}");
                let _ = display::fill_solid(&mut display, Rgb565::RED);
                hang().await
            }
        };

        frame += 1;
        if frame == 1 {
            // Deterministic CI/QEMU success marker: one full emulation
            // frame completed and was presented.
            esp_println::println!("NES_ESP32S3_BOOT_OK");
        }
        if frame % LOG_EVERY_N_FRAMES == 0 {
            esp_println::println!("nes: frame {frame}");
        }

        let elapsed = frame_start.elapsed();
        if elapsed < FRAME_PERIOD {
            Timer::after(FRAME_PERIOD - elapsed).await;
        }
    }
}

/// Parks the system. Used for unrecoverable init failures; the display
/// already shows what went wrong.
#[cfg(not(feature = "qemu"))]
async fn hang() -> ! {
    loop {
        Timer::after(Duration::from_secs(3600)).await;
    }
}

/// QEMU-only synchronous entry point.
///
/// The ESP32-S3 QEMU machine does not model the analog PLL (see the
/// `enable_pll_clk_impl` timeout patch) nor the timer interrupt that
/// `esp_rtos::start` needs, so the RTOS scheduler cannot run here.  This
/// variant skips the RTOS entirely and drives the NES core synchronously:
/// it is only a CI boot test that proves the firmware links, the ROM
/// loads, and the frame loop iterates.
#[cfg(feature = "qemu")]
#[esp_hal::main]
fn main() -> ! {
    heap_allocator!(size: board::HEAP_BYTES);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_println::println!("nes: boot");

    // TimerGroup is created for API parity, but the RTOS scheduler is
    // intentionally not started (see above).
    let _timg0 = TimerGroup::new(peripherals.TIMG0);

    let mut display = match display::init(peripherals.SPI2) {
        Ok(display) => display,
        Err(_) => qemu_hang(),
    };
    let pad = input::Pad::player_one();
    let mut audio = match audio::init(peripherals.I2S0, peripherals.DMA_CH0) {
        Ok(audio) => audio,
        Err(_) => qemu_hang(),
    };
    esp_println::println!("nes: audio ok");
    let mut dma_buf = match esp_hal::dma_tx_buffer!(audio::DMA_BYTES) {
        Ok(buf) => buf,
        Err(_) => qemu_hang(),
    };

    let mut driver = Box::new(EmuDriver::new());
    if driver.load_rom(rom::ROM).is_err() {
        esp_println::println!("nes: rom load failed");
        let _ = display::fill_solid(&mut display, Rgb565::RED);
        qemu_hang();
    }
    esp_println::println!("nes: rom loaded, {} bytes", rom::ROM.len());

    let mut frame: u64 = 0;
    loop {
        driver.set_controllers(pad.poll(), 0);
        if driver.step_frame().is_err() {
            esp_println::println!("nes: step_frame failed at frame {frame}");
            let _ = display::fill_solid(&mut display, Rgb565::RED);
            qemu_hang();
        }
        if display::present(&mut display, driver.frame_rgb565()).is_err() {
            esp_println::println!("nes: present failed at frame {frame}");
            qemu_hang();
        }
        // In QEMU `push_chunk` skips the DMA transfer; run its synchronous
        // buffer-fill part only.
        dma_buf = match audio.push_chunk_sync(driver.audio_chunk(), dma_buf) {
            Ok(buf) => buf,
            Err(e) => {
                esp_println::println!("nes: audio failed at frame {frame}: {e:?}");
                let _ = display::fill_solid(&mut display, Rgb565::RED);
                qemu_hang();
            }
        };

        frame += 1;
        if frame == 1 {
            esp_println::println!("NES_ESP32S3_BOOT_OK");
        }
        if frame % LOG_EVERY_N_FRAMES == 0 {
            esp_println::println!("nes: frame {frame}");
        }
        // No frame pacing in QEMU: iterate as fast as the emulator allows.
    }
}

/// QEMU-only park: the RTOS timer never runs here, so just spin.
#[cfg(feature = "qemu")]
fn qemu_hang() -> ! {
    loop {
        core::hint::spin_loop();
    }
}
