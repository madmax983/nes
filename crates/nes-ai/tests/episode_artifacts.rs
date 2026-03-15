use nes_ai::episode::{EpisodeArtifactWriter, EpisodeMetadata};
use nes_core::tas::{TasFrameRun, TasMovie};
use tempfile::tempdir;

#[test]
fn episode_writer_emits_metadata_and_tas_json() {
    let dir = tempdir().unwrap();
    let writer = EpisodeArtifactWriter::new(dir.path().to_path_buf());
    let movie = TasMovie::from_runs(vec![TasFrameRun::new(0, 0, 2)]);
    let meta = EpisodeMetadata {
        profile_id: "smb-control".to_owned(),
        snapshot_id: "smb-control-v1".to_owned(),
        rom_hash: "rom-hash".to_owned(),
        total_reward: 1.5,
        episode_frames: 2,
        final_state_hash: 42,
    };

    let paths = writer.write("eval", &movie, &meta).unwrap();
    assert!(paths.tas_json_path.exists());
    assert!(paths.run_json_path.exists());
    assert!(
        paths
            .macro_txt_path
            .as_ref()
            .is_some_and(|path| path.exists())
    );
}

#[test]
fn episode_writer_rejects_path_like_prefixes() {
    let dir = tempdir().unwrap();
    let writer = EpisodeArtifactWriter::new(dir.path().to_path_buf());
    let movie = TasMovie::from_runs(vec![TasFrameRun::new(0, 0, 2)]);
    let meta = EpisodeMetadata {
        profile_id: "smb-control".to_owned(),
        snapshot_id: "smb-control-v1".to_owned(),
        rom_hash: "rom-hash".to_owned(),
        total_reward: 1.5,
        episode_frames: 2,
        final_state_hash: 42,
    };

    let err = writer.write("../escape", &movie, &meta).unwrap_err();
    assert!(err.to_string().contains("invalid artifact prefix"));
}
