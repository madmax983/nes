import sys

def insert_docs(filepath, struct_name, docstring):
    with open(filepath, 'r') as f:
        content = f.read()

    lines = content.split('\n')
    out_lines = []
    inserted = False

    for i, line in enumerate(lines):
        if line.startswith(f"pub struct {struct_name}"):
            # Go back to before derive macros if any
            insert_idx = len(out_lines)
            while insert_idx > 0 and out_lines[insert_idx - 1].startswith("#["):
                insert_idx -= 1

            # Insert the docstring
            for doc_line in docstring.split('\n'):
                out_lines.insert(insert_idx, doc_line)
                insert_idx += 1
            inserted = True

        out_lines.append(line)

    if inserted:
        with open(filepath, 'w') as f:
            f.write('\n'.join(out_lines))
        print(f"Successfully inserted docs for {struct_name} in {filepath}")
    else:
        print(f"Failed to find {struct_name} in {filepath}")

if __name__ == "__main__":
    filepath = sys.argv[1]
    struct_name = sys.argv[2]
    docstring = sys.stdin.read()
    insert_docs(filepath, struct_name, docstring.strip())
