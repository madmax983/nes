import re

with open('crates/nes-mcp/src/protocol.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn dispatch_output_value',
    '#[must_use]\npub fn dispatch_output_value'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn tool_input_schema',
    '#[must_use]\npub fn tool_input_schema'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn jsonrpc_result',
    '#[must_use]\npub fn jsonrpc_result'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn jsonrpc_error',
    '#[must_use]\npub fn jsonrpc_error'
)

with open('crates/nes-mcp/src/protocol.rs', 'w') as f:
    f.write(content)
