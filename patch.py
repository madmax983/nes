with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()

# I removed should_resume_after_rewind_hold but it seems it's still needed in main.rs or input.rs.
# wait, should_resume_after_rewind_hold is used in main.rs: KeyboardDecision::SetRewindHeld
# Let's check where SetRewindHeld is handled.

data = data.replace("fn should_log_rollback(distance: u64) -> bool {", """fn should_resume_after_rewind_hold(held: bool) -> bool {
    !held
}

fn should_log_rollback(distance: u64) -> bool {""")

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
