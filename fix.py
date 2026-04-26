import sys

def main():
    content = open("crates/nes-test-harness/tests/bbbradsmith_golden_capture_cli.rs").read()

    content = content.replace("make_unreadable(&rom_path);", "    make_unreadable(&rom_path);")
    content = content.replace("make_readable(&rom_path);", "    make_readable(&rom_path);")
    content = content.replace("make_dir_unreadable(&suite);", "    make_dir_unreadable(&suite);")
    content = content.replace("make_dir_readable(&suite);", "    make_dir_readable(&suite);")
    content = content.replace("make_dir_unwritable(&golden);", "    make_dir_unwritable(&golden);")
    content = content.replace("make_dir_readable(&golden);", "    make_dir_readable(&golden);")

    open("crates/nes-test-harness/tests/bbbradsmith_golden_capture_cli.rs", "w").write(content)

if __name__ == "__main__":
    main()
