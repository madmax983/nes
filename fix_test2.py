with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

content = content.replace("fn execute_app_action_branches_return_expected_results", "use super::*; fn execute_app_action_branches_return_expected_results")

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)
