import re
with open('crates/nes-desktop/src/main.rs', 'r') as f:
    content = f.read()

# The file is currently on the failing branch, so it still has the macro.
# We need to revert the file to HEAD~1
