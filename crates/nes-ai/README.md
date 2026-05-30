# `nes-ai`

`nes-ai` is the Burn-based training crate for fixed-snapshot NES control tasks. The current v1 path is SMB control from a prepared World 1-1 snapshot, with replay artifacts written as TAS movies first and macro scripts second.

## Snapshot Prep

Generate the fixed control snapshot locally from the bootstrap TAS:

```powershell
cargo run -p nes-ai --bin prepare_smb_control -- `
  "./roms/homebrew/homebrew.nes" `
  ./crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json `
  ./artifacts/ai/snapshots/smb-1-1-control.state.json
```

Then copy the example configuration:

```powershell
cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml
```

Then point `rom_path` / `snapshot_path` at your local files.

## Training

Train from the local profile and optionally write checkpoints plus evaluation artifacts:

```powershell
cargo run -p nes-ai --bin train_smb_control -- `
  ./config/ai/profiles/smb-control.toml `
  4 `
  ./artifacts/ai/checkpoints `
  ./artifacts/ai/eval
```

Arguments:

- `profile_toml`: validated `AiProfileConfig`
- `episodes`: evaluation episodes after training
- `checkpoint_dir`: optional output directory for Burn checkpoints
- `artifact_dir`: optional output directory for evaluation artifacts

Checkpoints are saved as Burn recorder files with the recorder extension appended automatically, so `policy-update-0002` becomes `policy-update-0002.mpk`.

## Evaluation

Evaluate a saved checkpoint base path:

```powershell
cargo run -p nes-ai --bin eval_smb_control -- `
  ./config/ai/profiles/smb-control.toml `
  ./artifacts/ai/checkpoints/policy-update-0002 `
  2 `
  ./artifacts/ai/eval
```

If you omit checkpoint handling in library code, `evaluate_smb_control` falls back to a random baseline.

## Artifact Layout

Each evaluated episode can emit:

- `*.tas.json`: canonical replay movie
- `*.run.json`: metadata including reward total and final state hash
- `*.macro.txt`: compatibility export for legacy macro playback when supported

## Replay

Preferred replay path:

1. Load the emitted `TasMovie` JSON with `nes_core::tas`.
2. Replay it deterministically against `NesCore`.

Compatibility path:

1. Use the emitted `*.macro.txt`.
2. Feed it through the existing macro tooling for watched playbacks.
