//! Configuration resolution for commands

use crate::{
    config::{Config, ConfigMerger},
    error::Result,
    types::FunctionIdentity,
};
use std::path::Path;

/// Resolves configuration for a given runnable
pub struct ConfigResolver<'a> {
    project_root: Option<&'a Path>,
    identity: &'a FunctionIdentity,
}

impl<'a> ConfigResolver<'a> {
    pub fn new(project_root: Option<&'a Path>, identity: &'a FunctionIdentity) -> Self {
        Self {
            project_root,
            identity,
        }
    }

    pub fn resolve(&self) -> Result<Config> {
        let mut merger = ConfigMerger::new();

        // Load configs based on the runnable's location
        if let Some(ref file_path) = self.identity.file_path {
            merger.load_configs_for_path(file_path)?;
            
            // If we have a project root that's different from the file path, load it too
            if let Some(root) = self.project_root {
                if let Some(parent) = file_path.parent() {
                    if root != parent {
                        merger.load_configs_for_path(root)?;
                    }
                }
            }
        } else if let Some(root) = self.project_root {
            // No file path but we have project root
            merger.load_configs_for_path(root)?;
        }

        Ok(merger.get_merged_config())
    }
}