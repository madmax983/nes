#!/bin/bash
sed -i '$d' crates/nes-relay/src/main.rs
cat << 'TEST_EOF' >> crates/nes-relay/src/main.rs

    #[test]
    fn seed_entropy_returns_valid_non_zero_value() {
        let seed = super::seed_entropy();
        assert_ne!(seed, 0, "seed_entropy should not return exactly 0");
        assert_ne!(seed, 1, "seed_entropy should not return exactly 1");
    }
}
TEST_EOF
