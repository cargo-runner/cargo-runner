use super::FileType;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents the identity of a function in a Rust project
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionIdentity {
    pub package: Option<String>,
    pub module_path: Option<String>,
    pub file_path: Option<PathBuf>,
    pub function_name: Option<String>,
    pub file_type: Option<FileType>,
}

/// Match a stored field against a lookup field.
///
/// - If the stored identity omits the field, it is unconstrained.
/// - If the lookup omits the field, treat it as a **wildcard** (do not reject).
///   This is critical: CLI writes `package` when known, but builders often
///   look up without a package name — those overrides must still apply.
/// - If both are set, they must be equal.
impl FunctionIdentity {
    /// Check if this (stored override) identity matches a lookup identity.
    ///
    /// Returns true when every field present on **both** sides agrees. Fields
    /// present only on the stored side or only on the lookup side do not fail
    /// the match (partial / wildcard semantics).
    pub fn matches(&self, other: &FunctionIdentity) -> bool {
        if let (Some(my_val), Some(other_val)) = (&self.package, &other.package)
            && my_val != other_val
        {
            return false;
        }
        if let (Some(my_val), Some(other_val)) = (&self.module_path, &other.module_path)
            && my_val != other_val
        {
            return false;
        }
        if let (Some(my_val), Some(other_val)) = (&self.function_name, &other.function_name)
            && my_val != other_val
        {
            return false;
        }
        if let (Some(my_val), Some(other_val)) = (&self.file_type, &other.file_type)
            && my_val != other_val
        {
            return false;
        }
        if let (Some(my_file), Some(other_file)) = (&self.file_path, &other.file_path)
            && !paths_match(my_file, other_file)
        {
            return false;
        }

        true
    }
}

/// Check if two paths match, handling relative vs absolute paths
fn paths_match(path1: &Path, path2: &Path) -> bool {
    // If both are the same, quick return
    if path1 == path2 {
        return true;
    }

    // If one is absolute and one is relative, check if the relative path
    // is a suffix of the absolute path
    if path1.is_absolute() && path2.is_relative() {
        path1.ends_with(path2)
    } else if path2.is_absolute() && path1.is_relative() {
        path2.ends_with(path1)
    } else {
        // Both absolute or both relative, and they're not equal
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileType;
    use std::path::PathBuf;

    #[test]
    fn package_on_stored_only_still_matches() {
        let stored = FunctionIdentity {
            package: Some("my-pkg".into()),
            function_name: Some("test_add".into()),
            file_path: Some(PathBuf::from("/abs/src/lib.rs")),
            file_type: Some(FileType::CargoProject),
            ..Default::default()
        };
        let lookup = FunctionIdentity {
            package: None, // builders often omit this
            function_name: Some("test_add".into()),
            file_path: Some(PathBuf::from("/abs/src/lib.rs")),
            file_type: Some(FileType::CargoProject),
            ..Default::default()
        };
        assert!(stored.matches(&lookup));
    }

    #[test]
    fn package_mismatch_rejects() {
        let stored = FunctionIdentity {
            package: Some("a".into()),
            function_name: Some("t".into()),
            ..Default::default()
        };
        let lookup = FunctionIdentity {
            package: Some("b".into()),
            function_name: Some("t".into()),
            ..Default::default()
        };
        assert!(!stored.matches(&lookup));
    }

    #[test]
    fn relative_and_absolute_file_paths_match() {
        // Windows only treats drive-letter paths as absolute; `/proj/...` is not.
        let absolute = if cfg!(windows) {
            PathBuf::from(r"C:\proj\src\lib.rs")
        } else {
            PathBuf::from("/proj/src/lib.rs")
        };
        let stored = FunctionIdentity {
            file_path: Some(PathBuf::from("src/lib.rs")),
            function_name: Some("t".into()),
            ..Default::default()
        };
        let lookup = FunctionIdentity {
            file_path: Some(absolute),
            function_name: Some("t".into()),
            ..Default::default()
        };
        assert!(stored.matches(&lookup));
    }
}
