#[test]
fn trace_error() {
    let result = cargo_runner::commands::run::run_command("nonexistent.rs", true);
    println!("Trace: {:?}", result.unwrap_err());
}
