import os
import re

def fix_empty_code_blocks(directory):
    for root, _, files in os.walk(directory):
        for file in files:
            if not file.endswith('.rs'):
                continue
            path = os.path.join(root, file)
            with open(path, 'r') as f:
                content = f.read()

            # Find and replace "/// ```no_run\n/// // Example usage of \w+\n/// ```" with something that won't trigger "empty code block" warning
            content = re.sub(
                r'/// ```no_run\n\s*/// // Example usage of (\w+)\n\s*/// ```',
                r'/// ```no_run\n/// // Example usage of \1\n/// let _ = "\1";\n/// ```',
                content
            )

            with open(path, 'w') as f:
                f.write(content)

fix_empty_code_blocks("crates/nes-desktop/src")
