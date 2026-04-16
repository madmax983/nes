with open("crates/nes-desktop/src/main.rs", "r") as f:
    for i, line in enumerate(f):
        if "None" in line and "};" in next(f) and "macro_rules! build_ctx {" in next(f):
            print(f"Found it around line {i}")
            break
