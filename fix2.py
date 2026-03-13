with open('crates/nes-desktop/src/main.rs', 'r') as f:
    code = f.read()

code = code.replace("rta_manager: &Option<crate::rta::RtaManager>,", "rta_manager: Option<&crate::rta::RtaManager>,")
code = code.replace("let table = build_startup_table(&runtime, &session, &step_mode, &rta_manager);", "let table = build_startup_table(&runtime, &session, &step_mode, rta_manager.as_ref());")

with open('crates/nes-desktop/src/main.rs', 'w') as f:
    f.write(code)
