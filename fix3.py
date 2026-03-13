with open('crates/nes-desktop/src/main.rs', 'r') as f:
    code = f.read()

code = code.replace("rta_manager: &Option<RtaManager>,", "rta_manager: Option<&crate::rta::RtaManager>,")

with open('crates/nes-desktop/src/main.rs', 'w') as f:
    f.write(code)
