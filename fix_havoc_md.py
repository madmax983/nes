import os
import re

with open(".jules/havoc.md", "r") as f:
    content = f.read()

# We need to find the duplicate block and remove it. The bad one is:
# ## YYYY-MM-DD - Desktop RTA Profiles OOM via /dev/zero
# 🧨 **The Trigger:**  provided to  which iterates files and reads them via .
# 📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
# 🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_rta_profiles_oom -- --ignored`
# 😈 **Comment:** You assumed RTA profiles would only ever be finite files. You were wrong.

bad_block = """## YYYY-MM-DD - Desktop RTA Profiles OOM via /dev/zero
🧨 **The Trigger:**  provided to  which iterates files and reads them via .
📉 **The Stack Trace:** (Process sent SIGKILL due to Out of Memory. No explicit panic trace).
🧪 **Reproduction:** Run `cargo test -p nes-desktop havoc_rta_profiles_oom -- --ignored`
😈 **Comment:** You assumed RTA profiles would only ever be finite files. You were wrong.

"""

content = content.replace(bad_block, "")

with open(".jules/havoc.md", "w") as f:
    f.write(content)
