**👺 Havoc: `nes-desktop` Exits with 0 (Success) on Invalid CLI Arguments**

🧨 **The Trigger:**
When passing an invalid argument (e.g. `--invalid-flag-that-does-not-exist`) to the `nes-desktop` executable, the application parses the flags, prints an error message (`unknown flag: ...`), and then exits gracefully with a standard status code of `0`.

📉 **The Stack Trace:**
(No panic, but invalid standard exit code observed via integration test.)
```
unknown flag: --invalid-flag-that-does-not-exist
test havoc_crash_nes_desktop_returns_0_on_invalid_args ... ok
```

🧪 **Reproduction:**
```bash
cargo run -p nes-desktop -- --invalid-flag-that-does-not-exist
echo $? # Prints 0
```
Or run the Havoc target:
```bash
cargo test -p nes-desktop --test havoc -- --ignored
```

😈 **Comment:**
"You assumed your application would properly communicate failure to the OS. You were wrong. Automation scripts will assume everything went fine while your app did nothing."
