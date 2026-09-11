# nes-embassy

Hardware-neutral `no_std` adapter that runs `nes-core` on tiny Embassy targets
(ESP32-S3 and friends).

## Layout

- `src/` — `EmuDriver`: owns the core plus reusable RGB565 video and audio
  buffers. `no_std` + `alloc`; the only dependency is `nes-core`. Host tests
  in `tests/` run the exact same code firmware uses.
- `firmware/esp32-s3/` — reference firmware (own detached workspace,
  Xtensa-only): `esp-rtos` 0.4 startup, SPI LCD via `mipidsi`, I²S audio,
  GPIO buttons, board pin map. Not a workspace member, so host `cargo test`
  never tries to build it.

## Firmware loop

```rust
loop {
    let (p1, p2) = read_buttons();          // platform
    driver.set_controllers(p1, p2);
    driver.step_frame()?;                   // 1/60 s of emulation
    display.write_frame(driver.frame_rgb565()).await?;
    audio.push(driver.audio_chunk()).await?;
    Timer::after_millis(16).await;          // pace to 60 Hz
}
```

`step_frame` never allocates: both frame buffers are boxed once in
`EmuDriver::new`.
