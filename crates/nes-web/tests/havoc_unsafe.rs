use nes_web::NesWebEmulator;

#[test]
#[ignore = "Havoc OOB Read"]
fn havoc_web_frame_rgba_ptr_out_of_bounds() {
    let emulator = NesWebEmulator::new();

    let ptr = emulator.frame_rgba_ptr();
    let bad_len = emulator.frame_rgba_len() + 1000000;

    // This simulates what JS could do: read out of bounds from the exposed pointer
    let slice = unsafe { std::slice::from_raw_parts(ptr, bad_len) };

    // Read an element far out of bounds. This will likely segfault or read garbage.
    let _ = slice[bad_len - 1];
}
