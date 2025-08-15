//\! this is a stand alone rust file.
//\! used for testing cargo runner

use std::io;

fn add_two_numbers(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply_numbers(a: i32, b: i32) -> i32 {
    a * b
}

fn get_user_input() -> i32 {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read input");
    input.trim().parse().expect("Invalid number")
}

fn main() {
    println\!("Simple Rust Calculator\!");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn add_two_numbers() {
        assert_eq\!(add_two_numbers(2, 3), 5);
    }

    #[test]
    fn multiply_numbers() {
        assert_eq\!(multiply_numbers(2, 3), 6);
    }
}
EOF < /dev/null