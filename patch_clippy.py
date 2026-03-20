import re

content = open("crates/nes-rewind/src/worker.rs").read()
content = content.replace("while let Ok(_) = tm.rx.try_recv() {}", "while tm.rx.try_recv().is_ok() {}")
open("crates/nes-rewind/src/worker.rs", "w").write(content)
