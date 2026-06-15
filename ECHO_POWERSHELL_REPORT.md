# 🗣️ Echo: Commands fail in bash terminal

**Description:**

* 🤦 **The Confusion:** Tried to run the `train_smb_control` AI command from the `README.md`. I literally copy-pasted the command block into my terminal (I use Bash on Ubuntu like a normal person). Got a massive error about "command substitution" and none of it worked.

* 🕵️ **The Reality:** The README uses PowerShell syntax (` powershell -NoProfile...`) and backticks (\`) for line continuations. If I copy-paste backticks into bash, it tries to execute the text between them as a command.

* 💡 **The Fix:** Provide standard bash/sh equivalent commands using `\` for line continuations instead of assuming everyone has PowerShell installed, or remove the line continuations entirely so it's just one line.
