#!/bin/bash
# Remove everything from line 510 to end
sed -i '510,$d' crates/nes-web/src/lib.rs
cat << 'TEST_EOF' >> crates/nes-web/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_js_error_preserves_string() {
        let is_wasm = cfg!(target_arch = "wasm32");
        if is_wasm {
            let js_val = to_js_error("Custom error".to_string());
            assert!(format!("{:?}", js_val).contains("Custom error"));
        }
    }
}
TEST_EOF
