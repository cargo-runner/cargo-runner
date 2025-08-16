//! Example Trunk WASM application

use wasm_bindgen::prelude::*;

fn main() {
    // Trunk WASM app
    console_log!("Hello from WASM!");
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}