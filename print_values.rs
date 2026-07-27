fn main() {
    let raw = "PZLGITYE";
    // alphabet: A=0 P=1 Z=2 L=3 G=4 I=5 T=6 Y=7 E=8 O=9 X=10 U=11 K=12 S=13 V=14 N=15
    // P=1 Z=2 L=3 G=4 I=5 T=6 Y=7 E=8
    let digits: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    // check:
    let char0 = 'P';
    // 'P' is at index 1

    let address = 0x8000
        | (u16::from(digits[3] & 0x7) << 12)
        | (u16::from(digits[5] & 0x7) << 8)
        | (u16::from(digits[4] & 0x8) << 8)
        | (u16::from(digits[2] & 0x7) << 4)
        | (u16::from(digits[1] & 0x8) << 4)
        | u16::from(digits[4] & 0x7)
        | u16::from(digits[3] & 0x8);

    let value = ((digits[1] & 0x7) << 4)
        | ((digits[0] & 0x8) << 4)
        | (digits[0] & 0x7)
        | (digits[5] & 0x8);

    let compare = ((digits[7] & 0x7) << 4)
        | ((digits[6] & 0x8) << 4)
        | (digits[6] & 0x7)
        | (digits[7] & 0x8);

    println!("addr={} value={} compare={}", address, value, compare);
}
