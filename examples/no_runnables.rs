// This is an example file with no test functions or main()
// It should trigger the fallback command

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

fn main() {
    let x = add(1, 2);
    println!("x = {}", x);
    let y = multiply(3, 4);
    println!("y = {}", y);
}
