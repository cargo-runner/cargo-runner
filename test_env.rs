fn main() {
    println!("CARGO_TARGET_DIR = {:?}", std::env::var("CARGO_TARGET_DIR"));
    println!("All env vars:");
    for (key, value) in std::env::vars() {
        if key.starts_with("CARGO") || key.starts_with("RUST") {
            println!("  {} = {}", key, value);
        }
    }
}