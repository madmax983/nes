#[cfg(feature = "mcp-host")]
#[cfg(test)]
mod tests {
    use nes_desktop::mcp_host::read_framed_message;
    use std::io::{BufRead, Read};

    struct InfiniteLoopDetector {
        iterations: usize,
    }

    impl Read for InfiniteLoopDetector {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            self.iterations += 1;
            if self.iterations > 10 {
                panic!("Infinite loop detected!");
            }
            Ok(0) // Always return EOF
        }
    }

    impl BufRead for InfiniteLoopDetector {
        fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
            self.iterations += 1;
            if self.iterations > 10 {
                panic!("Infinite loop detected!");
            }
            Ok(&[]) // Always return EOF
        }

        fn consume(&mut self, _amt: usize) {}
    }

    #[test]
    fn test_read_framed_message_handles_eof() {
        let mut mock_reader = InfiniteLoopDetector { iterations: 0 };
        // The unmutated version will return Ok(None) immediately.
        // The mutated version (`read != 0`) will loop, hit iterations > 10, and panic!
        let result = read_framed_message(&mut mock_reader);
        assert!(matches!(result, Ok(None)));
        assert_eq!(mock_reader.iterations, 1); // Should only read once
    }
}
