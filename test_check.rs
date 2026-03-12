fn main() {
    let mut dummy_window = [1_u8; 8192];
    dummy_window[0] = 42;
    dummy_window[1024] = 43;

    let mut chr_data = vec![0_u8; 8192];
    chr_data[0..8192].copy_from_slice(&dummy_window);
    println!("chr_data[0]: {}", chr_data[0]);
}
