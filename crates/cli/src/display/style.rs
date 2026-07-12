//! Global display preferences (quiet / no-emoji) for human CLI output.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    static QUIET: Cell<bool> = const { Cell::new(false) };
    static NO_EMOJI: Cell<bool> = const { Cell::new(false) };
}

/// Process-wide: when true, failures in JSON/IDE modes print structured ErrorOutput.
static JSON_ERROR_MODE: AtomicBool = AtomicBool::new(false);

pub fn init(quiet: bool, no_emoji: bool) {
    let env_no_emoji = std::env::var_os("CARGO_RUNNER_NO_EMOJI").is_some()
        || std::env::var_os("NO_EMOJI").is_some();
    let env_quiet = std::env::var_os("CARGO_RUNNER_QUIET").is_some();
    QUIET.with(|c| c.set(quiet || env_quiet));
    NO_EMOJI.with(|c| c.set(no_emoji || env_no_emoji));
}

pub fn is_quiet() -> bool {
    QUIET.with(|c| c.get())
}

pub fn no_emoji() -> bool {
    NO_EMOJI.with(|c| c.get())
}

pub fn set_json_error_mode(on: bool) {
    JSON_ERROR_MODE.store(on, Ordering::SeqCst);
}

pub fn json_error_mode() -> bool {
    JSON_ERROR_MODE.load(Ordering::SeqCst)
}

/// Return `emoji` unless no-emoji mode is active (then empty).
pub fn icon(emoji: &str) -> &str {
    if no_emoji() { "" } else { emoji }
}

/// Prefix a human banner; omits emoji when disabled.
pub fn banner(emoji: &str, message: &str) -> String {
    if no_emoji() || emoji.is_empty() {
        message.to_string()
    } else {
        format!("{emoji} {message}")
    }
}

/// Strip common CLI emojis for --no-emoji output of legacy strings.
pub fn strip_emojis(s: &str) -> String {
    const EMOJIS: &[&str] = &[
        "🔍", "✅", "❌", "📌", "👀", "⚠️", "🚀", "📄", "📍", "🧪", "🔧", "📦", "📏", "📁", "🎯",
        "💡", "📝", "🎉", "⭐", "🔗", "✨", "🔥", "👉", "ℹ️", "❗️",
    ];
    let mut out = s.to_string();
    for e in EMOJIS {
        out = out.replace(e, "");
    }
    // Collapse double spaces left by removals
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().to_string()
}

/// println! that respects no-emoji (strips known emojis from the line).
pub fn println_human(s: impl AsRef<str>) {
    let s = s.as_ref();
    if no_emoji() {
        println!("{}", strip_emojis(s));
    } else {
        println!("{s}");
    }
}

pub fn eprintln_human(s: impl AsRef<str>) {
    if is_quiet() {
        return;
    }
    let s = s.as_ref();
    if no_emoji() {
        eprintln!("{}", strip_emojis(s));
    } else {
        eprintln!("{s}");
    }
}
