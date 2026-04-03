macro_rules! my_macro {
    () => {
        x + 1
    }
}

fn main() {
    let x = 5;
    println!("{}", my_macro!());
}
