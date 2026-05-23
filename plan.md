1. **Verify workspace changes**:
   - I will run `git status` and `git diff` to verify the existing workspace changes for `split_csv` optimizations in `crates/nes-dsl/src/parser.rs` and `crates/nes-dsl/src/lib.rs`.
2. **Add test for capacity calculation and clean up artifacts**:
   - I will use the following Heredoc to append the `test_csv_allocation` test to `crates/nes-dsl/src/parser.rs`, and then remove all temporary patch and reject files:
     ```bash
     cat << 'EOF' > test_parse_csv.patch
     --- crates/nes-dsl/src/parser.rs
     +++ crates/nes-dsl/src/parser.rs
     @@ -395,6 +395,12 @@
          }

          #[test]
     +    fn test_csv_allocation() {
     +        let parts = split_csv(r#"A, B, C"#).expect("csv parse");
     +        assert_eq!(parts.capacity(), 3);
     +    }
     +
     +    #[test]
          fn csv_and_string_literal_helpers_handle_escapes_and_errors() {
              let parts = split_csv(r#""A,B", $10, "C\"D""#).expect("csv parse");
              assert_eq!(parts, vec![r#""A,B""#, "$10", r#""C\"D""#]);
     EOF
     patch crates/nes-dsl/src/parser.rs test_parse_csv.patch
     rm -f test_parse_csv.patch update_parser.patch update_lib.patch crates/nes-dsl/src/parser.rs.rej
     ```
3. **Verify the test addition**:
   - I will run `git diff crates/nes-dsl/src/parser.rs` to verify the test was added correctly.
4. **Format code**:
   - Run `cargo fmt --all`.
5. **Run tests and linters**:
   - Run `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --workspace` to ensure all tests pass.
6. **Update Journal**:
   - I will update `.jules/bolt.md` using the following bash command to strictly follow the Bolt persona format:
     ```bash
     cat << 'EOF' >> .jules/bolt.md

     **[Eliminating Vec Reallocations in CSV Parsing]**
     **Learning:** [When parsing comma-separated lists, using `Vec::new()` causes intermediate heap allocations as the vector grows. Counting the number of commas beforehand provides an accurate capacity estimate.]
     **Action:** [Use `input.chars().filter(|&c| c == ',').count() + 1` to pre-allocate the vector with `Vec::with_capacity()` and eliminate O(N) reallocations during parsing.]
     EOF
     ```
7. **Complete pre-commit steps**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
8. **Submit the Pull Request**:
   - I will submit the changes to the `main` branch. The PR title will be "⚡ Bolt: [Eliminate Vec reallocations in split_csv]" and the description will include `💡 What`, `🎯 Why`, `📊 Impact`, and `🔬 Measurement`.
