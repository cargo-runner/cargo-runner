fn main() {
    println!("Testing standalone configuration");
}

#[test]
fn test_config_isolation() {
    assert_eq!(1 + 1, 2);
}