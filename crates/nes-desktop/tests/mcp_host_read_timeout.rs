#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufRead, Read, Result as IoResult};

// A custom reader that panics if called repeatedly when read == 0.
struct SingleZeroReader {
    returned_zero: bool,
}

impl Read for SingleZeroReader {
    fn read(&mut self, _buf: &mut [u8]) -> IoResult<usize> {
        if self.returned_zero {
            panic!("Called read after returning 0! Infinite loop.");
        }
        self.returned_zero = true;
        Ok(0)
    }
}

impl BufRead for SingleZeroReader {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        if self.returned_zero {
            panic!("Called fill_buf after returning 0! Infinite loop.");
        }
        self.returned_zero = true;
        Ok(&[])
    }

    fn consume(&mut self, _amt: usize) {}
}

#[test]
fn test_read_framed_message_does_not_loop_infinitely_on_zero() {
    let mut reader = SingleZeroReader {
        returned_zero: false,
    };
    let result = read_framed_message(&mut reader);
    assert_eq!(result, Ok(None));
}
