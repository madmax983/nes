import re
content = open('crates/nes-dsl/src/lib.rs').read()
def get_parse_number(content):
    match = re.search(r'fn parse_number\b[^}]*}', content)
    if match:
        print(match.group(0))
get_parse_number(content)
