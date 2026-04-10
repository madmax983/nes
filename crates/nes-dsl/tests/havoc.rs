use nes_dsl::assemble;

#[test]
#[ignore]
fn havoc_macro_engine_oom() {
    // Generate a massive string to cause an OOM panic within the `assemble` function.
    // The assembler's String usage assumes memory allocation never fails, tearing
    // down the entire host application when the file size exceeds memory instead of
    // gracefully returning an I/O error to the caller.
    let mut script = String::with_capacity(usize::MAX);
    script.push_str("LDA #$00\n");
    let _ = assemble(&script);
}
