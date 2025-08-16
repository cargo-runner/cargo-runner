//! Example Dioxus application

use dioxus::prelude::*;

fn main() {
    // Dioxus app main
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        h1 { "Hello Dioxus!" }
    }
}