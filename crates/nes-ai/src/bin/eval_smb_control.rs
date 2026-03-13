fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = nes_ai::trainer::TrainerConfig::smoke();
    let summary = nes_ai::trainer::evaluate_random_policy(&cfg, 8)?;
    println!("average_return={}", summary.average_return);
    Ok(())
}
