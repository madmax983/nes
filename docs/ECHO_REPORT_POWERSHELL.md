# 🗣️ Echo: Snapshot Prep example in nes-ai fails on Linux/Bash

**Description:**

* 🤦 **The Confusion:** Tried to run the `prepare_smb_control` example from the `nes-ai/README.md`. Bash gave a weird syntax error: `unexpected EOF while looking for matching \`'`

* 🕵️ **The Reality:** The example uses PowerShell's backtick \` line continuations which breaks on Linux/Mac bash terminals when copy-pasted. Bash interprets the backtick as command substitution syntax.

* 💡 **The Fix:** Provide both powershell and bash examples, or just format the commands on a single line.
