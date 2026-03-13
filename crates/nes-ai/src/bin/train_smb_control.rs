fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = nes_ai::trainer::TrainerConfig::smoke();
    let summary = nes_ai::trainer::run_mock_ppo_smoke(&cfg, 8)?;
    println!("average_return={}", summary.average_return);
    Ok(())
}
