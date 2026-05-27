use nes_test_harness::read_pcm_i16le;
use std::path::Path;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_test_read_pcm_oom() {
    let _ = read_pcm_i16le(Path::new("/dev/zero"));
}
