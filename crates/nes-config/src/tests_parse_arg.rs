#[cfg(test)]
mod parse_arg_tests {
    use crate::parse_config_path_arg;

    // This test ensures that the while loop increments the index correctly.
    // In a mutated version where idx isn't incremented, `tx.send` will never be reached,
    // causing `rx.recv_timeout` to fail and panic.
    // Since we only want it to panic in the MUTATED version, we should just let it panic if timeout!
    // We remove #[should_panic]. When it times out, it panics and fails the test.
    #[test]
    fn parse_config_path_arg_terminates_on_bad_idx_mutation_timeout() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let args = vec!["--config".to_string(), "val".to_string()];
            let _ = parse_config_path_arg(&args);
            let _ = tx.send(());
        });

        // Wait up to 1 second. If it doesn't return, it means infinite loop!
        // We PANIC here so cargo-mutants considers the test FAILED (mutant caught!).
        if rx.recv_timeout(std::time::Duration::from_secs(1)).is_err() {
            panic!("Infinite loop detected in parse_config_path_arg / parse_arg");
        }
    }
}
