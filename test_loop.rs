use std::time::Duration;

fn main() {
    let start = std::time::Instant::now();
    let duration = Duration::from_millis(5000);

    let end = start + duration;

    println!("Elapsed check is valid: {}", start.elapsed() < duration);
}
