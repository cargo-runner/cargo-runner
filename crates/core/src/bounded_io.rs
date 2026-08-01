//! Bounded file reads for untrusted input.
//!
//! Source files, BUILD files and config all come from the repository being
//! inspected. Reading them without a ceiling lets a single pathological file
//! exhaust memory, and deeply nested source can additionally drive the
//! tree-sitter walkers (`scope_detector::visit_node`,
//! `find_doc_tests_recursive`) deep enough to overflow the stack. A size cap
//! bounds both, without a depth limit that would silently drop runnables from
//! legitimately deep code.

use std::path::Path;

/// Largest file cargo-runner will read. Rust sources, BUILD files and
/// `.cargo-runner.json` are all far below this in practice.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

/// `read_to_string`, refusing files above [`MAX_FILE_BYTES`].
pub fn read_to_string_capped(path: &Path) -> std::io::Result<String> {
    let len = std::fs::metadata(path)?.len();
    if len > MAX_FILE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "{} is {len} bytes, above the {MAX_FILE_BYTES}-byte limit",
                path.display()
            ),
        ));
    }
    std::fs::read_to_string(path)
}
