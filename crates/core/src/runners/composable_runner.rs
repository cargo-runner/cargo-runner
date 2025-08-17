//! Composable runner with dependency injection
//! 
//! This runner uses the interface traits to provide a flexible,
//! plugin-based architecture.

use std::path::Path;
use std::sync::Arc;
use std::cell::RefCell;

use crate::{
    build_system::BuildSystemDetector,
    command::CargoCommand,
    config::v2::{ConfigLoader, V2Config},
    error::Result,
    interfaces::{
        ExecutionContext, PathResolver, ModuleResolver, 
        RunnableDetector as RunnableDetectorTrait, TargetSelection,
    },
    types::Runnable,
    utils::detect_file_type,
};

/// A composable runner that uses dependency injection for flexibility
pub struct ComposableRunner {
    /// Path resolution service
    path_resolver: Arc<dyn PathResolver>,
    
    /// Module resolution service
    module_resolver: Arc<dyn ModuleResolver>,
    
    /// Runnable detection service
    runnable_detector: Arc<dyn RunnableDetectorTrait>,
    
    /// Target selection service
    target_selector: Arc<dyn TargetSelection>,
    
    /// Configuration
    config: V2Config,
    
    /// Parser for extracting scopes (wrapped for interior mutability)
    parser: RefCell<crate::parser::RustParser>,
}

impl ComposableRunner {
    /// Create a new composable runner with default services
    pub fn new() -> Result<Self> {
        Self::with_services(
            Arc::new(crate::services::DefaultPathResolver::new()),
            Arc::new(crate::services::TreeSitterModuleResolver::new()),
            Arc::new(crate::services::PatternRunnableDetector::new()?),
            Arc::new(crate::services::FileTargetSelector::new()),
        )
    }
    
    /// Create a runner with custom services (for testing or plugins)
    pub fn with_services(
        path_resolver: Arc<dyn PathResolver>,
        module_resolver: Arc<dyn ModuleResolver>,
        runnable_detector: Arc<dyn RunnableDetectorTrait>,
        target_selector: Arc<dyn TargetSelection>,
    ) -> Result<Self> {
        let config = ConfigLoader::load()
            .unwrap_or_else(|_| V2Config::default_with_build_system());
        
        Ok(Self {
            path_resolver,
            module_resolver,
            runnable_detector,
            target_selector,
            config,
            parser: RefCell::new(crate::parser::RustParser::new()?),
        })
    }
    
    /// Build execution context for a file
    pub fn build_context(&self, file_path: &Path, line: Option<u32>) -> Result<ExecutionContext> {
        // Read source code
        let source_code = std::fs::read_to_string(file_path)?;
        
        // Parse scopes
        let scopes = self.parser.borrow_mut().get_scopes(&source_code, file_path)?;
        let extended_scopes = self.parser.borrow_mut().get_extended_scopes(&source_code, file_path)?;
        
        // Determine project root
        let project_root = self.path_resolver
            .find_project_root(file_path)
            .unwrap_or_else(|| file_path.parent().unwrap_or(Path::new(".")).to_path_buf());
        
        // Get package name
        let package_name = self.get_package_name(file_path).ok();
        
        // Resolve module path
        let module_path = self.module_resolver.module_path_from_file(
            file_path,
            package_name.as_deref(),
        );
        
        // Detect file type
        let file_type = detect_file_type(file_path);
        
        // Detect build system
        let detector = crate::build_system::DefaultBuildSystemDetector;
        let build_system = detector.detect(&project_root)
            .unwrap_or(crate::build_system::BuildSystem::Cargo);
        
        Ok(ExecutionContext {
            project_root: project_root.clone(),
            working_directory: project_root,
            file_path: file_path.to_path_buf(),
            line_number: line,
            package_name: package_name.clone(),
            crate_name: package_name,
            build_system,
            linked_projects: self.config.linked_projects.clone().unwrap_or_default(),
            source_code,
            file_type,
            scopes,
            extended_scopes,
            module_path,
        })
    }
    
    /// Execute the runner to build a command
    pub fn execute(&self, file_path: &Path, line: Option<u32>) -> Result<CargoCommand> {
        // Build context
        let context = self.build_context(file_path, line)?;
        
        // Detect runnables
        let runnables = self.runnable_detector.detect(
            &context.scopes,
            &context.extended_scopes,
            &context.source_code,
            file_path,
        );
        
        // Select best runnable
        let target_runnable = self.runnable_detector.get_best_runnable(
            runnables.clone(),
            line,
        );
        
        // If no runnable found, try fallback
        let runnable = target_runnable.ok_or(crate::Error::NoRunnableFound)?;
        
        // Build command using the existing v2 config system
        // (This is a bridge until we have plugins fully working)
        self.build_command_for_runnable(&context, &runnable)
    }
    
    /// Build a command for a specific runnable
    fn build_command_for_runnable(
        &self,
        context: &ExecutionContext,
        runnable: &Runnable,
    ) -> Result<CargoCommand> {
        // Create scope context for v2 config
        let scope_context = crate::config::v2::ScopeContext {
            file_path: Some(context.file_path.clone()),
            crate_name: context.crate_name.clone(),
            module_path: if context.module_path.is_empty() {
                None
            } else {
                Some(context.module_path.clone())
            },
            function_name: runnable.get_function_name(),
            type_name: None,
            method_name: None,
            scope_kind: None,
        };
        
        // Use v2 config resolver
        let resolver = self.config.resolver();
        resolver
            .resolve_command(&scope_context, runnable.kind.clone())
            .map_err(|e| crate::Error::ConfigError(e))
    }
    
    /// Get package name from Cargo.toml
    fn get_package_name(&self, file_path: &Path) -> Result<String> {
        let project_root = self.path_resolver
            .find_project_root(file_path)
            .ok_or_else(|| crate::Error::Other("No project root found".to_string()))?;
        
        let cargo_toml = project_root.join("Cargo.toml");
        if !self.path_resolver.exists(&cargo_toml) {
            return Err(crate::Error::Other("No Cargo.toml found".to_string()));
        }
        
        let content = std::fs::read_to_string(&cargo_toml)?;
        // Simple regex-based extraction for package name
        // Avoids adding toml dependency just for this
        for line in content.lines() {
            if line.trim().starts_with("name") && line.contains('=') {
                if let Some(name_part) = line.split('=').nth(1) {
                    let name = name_part.trim().trim_matches('"').trim_matches('\'');
                    return Ok(name.to_string());
                }
            }
        }
        
        Err(crate::Error::Other("No package name found in Cargo.toml".to_string()))
    }
}

/// Builder for ComposableRunner with fluent API
pub struct ComposableRunnerBuilder {
    path_resolver: Option<Arc<dyn PathResolver>>,
    module_resolver: Option<Arc<dyn ModuleResolver>>,
    runnable_detector: Option<Arc<dyn RunnableDetectorTrait>>,
    target_selector: Option<Arc<dyn TargetSelection>>,
    config: Option<V2Config>,
}

impl ComposableRunnerBuilder {
    pub fn new() -> Self {
        Self {
            path_resolver: None,
            module_resolver: None,
            runnable_detector: None,
            target_selector: None,
            config: None,
        }
    }
    
    pub fn with_path_resolver(mut self, resolver: Arc<dyn PathResolver>) -> Self {
        self.path_resolver = Some(resolver);
        self
    }
    
    pub fn with_module_resolver(mut self, resolver: Arc<dyn ModuleResolver>) -> Self {
        self.module_resolver = Some(resolver);
        self
    }
    
    pub fn with_runnable_detector(mut self, detector: Arc<dyn RunnableDetectorTrait>) -> Self {
        self.runnable_detector = Some(detector);
        self
    }
    
    pub fn with_target_selector(mut self, selector: Arc<dyn TargetSelection>) -> Self {
        self.target_selector = Some(selector);
        self
    }
    
    pub fn with_config(mut self, config: V2Config) -> Self {
        self.config = Some(config);
        self
    }
    
    pub fn build(self) -> Result<ComposableRunner> {
        let path_resolver = self.path_resolver
            .unwrap_or_else(|| Arc::new(crate::services::DefaultPathResolver::new()));
        
        let module_resolver = self.module_resolver
            .unwrap_or_else(|| Arc::new(crate::services::TreeSitterModuleResolver::new()));
        
        let runnable_detector = self.runnable_detector
            .unwrap_or_else(|| Arc::new(crate::services::PatternRunnableDetector::new().unwrap()));
        
        let target_selector = self.target_selector
            .unwrap_or_else(|| Arc::new(crate::services::FileTargetSelector::new()));
        
        let config = self.config
            .unwrap_or_else(|| V2Config::default_with_build_system());
        
        Ok(ComposableRunner {
            path_resolver,
            module_resolver,
            runnable_detector,
            target_selector,
            config,
            parser: RefCell::new(crate::parser::RustParser::new()?),
        })
    }
}