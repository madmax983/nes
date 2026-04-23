# 🗣️ Echo: Netplay instructions are confusing

## 🤦 The Confusion
I wanted to try the rollback netplay. The `README.md` says to run:

```powershell
# Terminal 1: relay server
cargo run -p nes-relay -- --bind 0.0.0.0:4545

# Terminal 2: player 1
cargo run -p nes-desktop --release -- --netplay --netplay-relay <relay-host>:4545 --netplay-room river-city --netplay-player 1 ./roms/homebrew/homebrew.nes

# Terminal 3: player 2
cargo run -p nes-desktop --release -- --netplay --netplay-relay <relay-host>:4545 --netplay-room river-city --netplay-player 2 ./roms/homebrew/homebrew.nes
```

But when I run `cargo run -p nes-desktop --release -- --netplay --netplay-relay 127.0.0.1:4545 --netplay-room river-city --netplay-player 1 ./roms/homebrew/homebrew.nes`, nothing tells me what `<relay-host>` should be if I'm running locally. I had to guess it was `127.0.0.1` or `localhost`. The `README.md` should use `127.0.0.1` in the example to make it a copy-pasteable command.

## 🕵️ The Reality
The instructions are supposed to be copy-pasteable, but they use a placeholder `<relay-host>` instead of a sane default for local testing like `127.0.0.1`.

## 💡 The Fix
Change `<relay-host>` to `127.0.0.1` in the `README.md` netplay commands so people can literally copy-paste and test it locally.
