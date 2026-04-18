import re

def main():
    with open('crates/nes-desktop/src/netplay.rs', 'r') as f:
        content = f.read()

    old_test = """let mut engine = nes_netplay::RollbackEngine::new(nes_netplay::RollbackConfig { max_rollback_frames: 4, input_delay_frames: 0 }).unwrap();"""
    new_test = """let mut engine = nes_netplay::RollbackEngine::new(nes_netplay::RollbackConfig { max_rollback_frames: 4, input_delay_frames: 0, local_player: 1 }).unwrap();"""

    content = content.replace(old_test, new_test)

    with open('crates/nes-desktop/src/netplay.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
