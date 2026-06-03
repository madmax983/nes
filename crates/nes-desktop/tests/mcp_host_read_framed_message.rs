use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufRead, Read, Result as IoResult};

struct FailsOnLoop {
    calls: usize,
}

impl Read for FailsOnLoop {
    fn read(&mut self, _buf: &mut [u8]) -> IoResult<usize> {
        Ok(0)
    }
}

impl BufRead for FailsOnLoop {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        Ok(&[])
    }

    fn consume(&mut self, _amt: usize) {}

    fn read_line(&mut self, _buf: &mut String) -> IoResult<usize> {
        self.calls += 1;
        if self.calls > 3 {
            // Panic specifically so it CAUSES A TEST FAILURE, not an error!
            // Wait, an error from read_line becomes an `Err` from `read_framed_message`,
            // which causes the test to fail. Let's just panic to be certain.
            panic!("INFINITE LOOP MUTANT CAUGHT");
        }
        Ok(0) // EOF
    }
}

#[test]
fn test_read_framed_message_kill_mutant() {
    let mut reader = FailsOnLoop { calls: 0 };
    // This will normally return Ok(None) immediately.
    // If mutant `read != 0` is applied, it will loop and panic, failing the test.
    let _ = read_framed_message(&mut reader);
}
