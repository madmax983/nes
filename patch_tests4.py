import re

def main():
    with open('crates/nes-desktop/src/netplay.rs', 'r') as f:
        content = f.read()

    old_test = """let mut now = std::time::Instant::now();"""
    new_test = """let now = std::time::Instant::now();"""

    content = content.replace(old_test, new_test)

    with open('crates/nes-desktop/src/netplay.rs', 'w') as f:
        f.write(content)

if __name__ == '__main__':
    main()
