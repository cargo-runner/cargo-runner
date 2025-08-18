//! Configuration access trait for builders

// NUKE-CONFIG: Removed TestFramework import
use crate::{
    config::{Config, Features},
    types::FileType,
};
use std::collections::HashMap;

/// Trait for accessing configuration in a file-type aware manner
pub trait ConfigAccess {
    fn get_channel<'a>(&self, config: &'a Config, file_type: FileType) -> Option<&'a str> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.channel.as_deref(),
            _ => None, // rustc and single_file_script don't have channel
        }
    }

    fn get_features<'a>(&self, config: &'a Config, file_type: FileType) -> Option<&'a Features> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.features.as_ref(),
            _ => None, // rustc and single_file_script don't have features
        }
    }

    fn get_extra_args<'a>(
        &self,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a Vec<String>> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.extra_args.as_ref(),
            FileType::Standalone => None, // RustcConfig no longer has extra_args at top level
            FileType::SingleFileScript => config.single_file_script.as_ref()?.extra_args.as_ref(),
        }
    }

    fn get_extra_env<'a>(
        &self,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a HashMap<String, String>> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.extra_env.as_ref(),
            FileType::Standalone => None, // RustcConfig no longer has extra_env at top level
            FileType::SingleFileScript => config.single_file_script.as_ref()?.extra_env.as_ref(),
        }
    }

    fn get_extra_test_binary_args<'a>(
        &self,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a Vec<String>> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.extra_test_binary_args.as_ref(),
            _ => None, // rustc and single_file_script don't have test binary args
        }
    }

    fn get_linked_projects<'a>(
        &self,
        config: &'a Config,
        file_type: FileType,
    ) -> Option<&'a Vec<String>> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.linked_projects.as_ref(),
            _ => None, // rustc and single_file_script don't have linked projects
        }
    }

    // NUKE-CONFIG: Removed get_test_framework and get_binary_framework methods
    // TODO: Add simple tool selection methods when implementing new config

    fn get_command<'a>(&self, config: &'a Config, file_type: FileType) -> Option<&'a str> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.command.as_deref(),
            _ => None, // rustc and single_file_script don't have custom command
        }
    }

    fn get_subcommand<'a>(&self, config: &'a Config, file_type: FileType) -> Option<&'a str> {
        match file_type {
            FileType::CargoProject => config.cargo.as_ref()?.subcommand.as_deref(),
            _ => None, // rustc and single_file_script don't have subcommand
        }
    }
}
