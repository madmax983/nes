import re
import glob

html_file = 'target/llvm-cov/html/coverage/app/crates/nes-dsl/src/lib.rs.html'
with open(html_file, 'r') as f:
    text = f.read()

unexecuted = re.findall(r'<td class=\'unexecuted.*?>(.*?)</td><td class=\'code.*?>(.*?)</td>', text)
for u in unexecuted:
    print(u)
