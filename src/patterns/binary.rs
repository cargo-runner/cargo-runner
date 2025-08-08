use crate::{
    error::Result,
    patterns::Pattern,
    types::{Runnable, RunnableKind, Scope, ScopeKind},
};
use std::path::Path;

pub struct BinaryPattern;

impl Pattern for BinaryPattern {
    fn detect(&self, scope: &Scope, _source: &str, file_path: &Path) -> Result<Option<Runnable>> {
        if let ScopeKind::Function = scope.kind {
            if scope.name.as_deref() == Some("main") {
                let bin_name = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());

                let runnable = Runnable {
                    label: if let Some(ref name) = bin_name {
                        format!("Run binary '{}'", name)
                    } else {
                        "Run main()".to_string()
                    },
                    scope: scope.clone(),
                    kind: RunnableKind::Binary { bin_name },
                    module_path: String::new(),
                    file_path: file_path.to_path_buf(),
                    extended_scope: None, // Will be filled by detector
                };
                return Ok(Some(runnable));
            }
        }
        Ok(None)
    }
}