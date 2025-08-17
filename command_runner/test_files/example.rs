// Example Rust file for testing the command runner

fn main() {
    println!("Hello from Rust!");
    let result = add(2, 3);
    println!("2 + 3 = {}", result);
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[test]
fn test_addition() {
    assert_eq!(add(2, 2), 4);
}

#[test]
fn test_subtraction() {
    assert_eq!(5 - 3, 2);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_negative() {
        assert_eq!(add(-1, 1), 0);
    }
    
    #[test]
    fn test_add_zero() {
        assert_eq!(add(0, 0), 0);
    }
}

#[bench]
fn bench_addition(b: &mut test::Bencher) {
    b.iter(|| {
        for i in 0..1000 {
            test::black_box(add(i, i));
        }
    });
}