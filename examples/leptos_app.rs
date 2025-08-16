//! Example Leptos application

use leptos::*;

fn main() {
    // Leptos app main
    println!("Running Leptos app!");
}

#[component]
fn App() -> impl IntoView {
    view! {
        <h1>"Hello Leptos!"</h1>
    }
}