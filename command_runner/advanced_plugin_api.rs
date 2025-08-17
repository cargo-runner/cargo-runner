//! Advanced Plugin API for Universal Command Runner
//! 
//! This provides a sophisticated plugin system with:
//! - Hierarchical scope resolution
//! - Pattern-based detector system
//! - Flexible command building with lambdas
//! - Module resolution customization

use std::path::{Path, PathBuf};
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use std::ops::Range;

// ============================================================================
// Core Scope System
// ============================================================================

/// Represents a scope in the source code with hierarchical priority
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub kind: ScopeKind,
    pub range: Range<usize>,
    pub line_range: Range<u32>,
    pub name: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Hierarchical scope types (default priority order)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeKind {
    File = 0,           // Lowest priority (widest scope)
    Module = 1,
    Namespace = 2,
    Class = 3,
    Structure = 4,
    Enumeration = 5,
    Union = 6,
    Trait = 7,
    Interface = 8,
    Implementation = 9,
    Method = 10,
    Function = 11,      // Highest priority (narrowest scope)
    Closure = 12,
    Block = 13,
}

/// Scope resolution configuration
pub struct ScopeConfig {
    /// Custom priority order (overrides default)
    pub priority_order: Option<Vec<ScopeKind>>,
    
    /// Whether to use scope size as tiebreaker
    pub use_size_tiebreaker: bool,
    
    /// Minimum scope size to consider (in lines)
    pub min_scope_size: u32,
}

impl Default for ScopeConfig {
    fn default() -> Self {
        Self {
            priority_order: None,
            use_size_tiebreaker: true,
            min_scope_size: 0,
        }
    }
}

/// Resolves the best scope for a given line number
pub struct ScopeResolver {
    config: ScopeConfig,
}

impl ScopeResolver {
    pub fn new(config: ScopeConfig) -> Self {
        Self { config }
    }
    
    /// Find the best scope for a given line number
    pub fn resolve_best_scope(&self, line: u32, scopes: &[Scope]) -> Option<Scope> {
        let mut matching_scopes: Vec<_> = scopes
            .iter()
            .filter(|s| s.line_range.contains(&line))
            .filter(|s| s.line_range.end - s.line_range.start >= self.config.min_scope_size)
            .collect();
        
        if matching_scopes.is_empty() {
            return None;
        }
        
        // Sort by priority
        if let Some(ref order) = self.config.priority_order {
            matching_scopes.sort_by_key(|s| {
                order.iter().position(|&k| k == s.kind).unwrap_or(usize::MAX)
            });
        } else {
            // Use default priority (higher enum value = higher priority)
            matching_scopes.sort_by_key(|s| std::cmp::Reverse(s.kind as u8));
        }
        
        // If same priority and size tiebreaker enabled, choose smallest scope
        if self.config.use_size_tiebreaker && matching_scopes.len() > 1 {
            let best_priority = matching_scopes[0].kind;
            let same_priority: Vec<_> = matching_scopes
                .iter()
                .take_while(|s| s.kind == best_priority)
                .collect();
            
            if let Some(smallest) = same_priority.iter()
                .min_by_key(|s| s.line_range.end - s.line_range.start)
            {
                return Some((*smallest).clone());
            }
        }
        
        matching_scopes.first().map(|s| (*s).clone())
    }
}

// ============================================================================
// Detector System
// ============================================================================

/// Pattern-based detector for identifying runnables
#[derive(Debug, Clone)]
pub struct Detector {
    pub id: String,
    pub patterns: Vec<DetectorPattern>,
    pub runnable_kind: RunnableKind,
    pub framework: Option<String>,
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub enum DetectorPattern {
    /// File path pattern (glob)
    FilePath(String),
    
    /// File name pattern
    FileName(String),
    
    /// Symbol/attribute pattern (e.g., #[test], @Test)
    Attribute(String),
    
    /// Function/method name pattern
    FunctionName(String),
    
    /// Module/namespace pattern
    ModuleName(String),
    
    /// Macro invocation (e.g., criterion_main!)
    MacroInvocation(String),
    
    /// Custom pattern with lambda
    Custom(Arc<dyn Fn(&DetectorContext) -> bool + Send + Sync>),
}

/// Context provided to detectors
#[derive(Debug, Clone)]
pub struct DetectorContext {
    pub file_path: PathBuf,
    pub scope: Scope,
    pub source_snippet: String,
    pub ast_node: Option<AstNodeInfo>,
    pub module_path: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AstNodeInfo {
    pub kind: String,
    pub name: Option<String>,
    pub attributes: Vec<String>,
    pub children: Vec<String>,
}

/// Detector matcher that evaluates patterns
pub struct DetectorMatcher {
    detectors: Vec<Detector>,
}

impl DetectorMatcher {
    pub fn new(detectors: Vec<Detector>) -> Self {
        Self { detectors }
    }
    
    pub fn match_detectors(&self, context: &DetectorContext) -> Vec<&Detector> {
        let mut matches: Vec<_> = self.detectors
            .iter()
            .filter(|d| self.matches_all_patterns(d, context))
            .collect();
        
        // Sort by priority (higher priority first)
        matches.sort_by_key(|d| std::cmp::Reverse(d.priority));
        matches
    }
    
    fn matches_all_patterns(&self, detector: &Detector, context: &DetectorContext) -> bool {
        detector.patterns.iter().all(|pattern| {
            match pattern {
                DetectorPattern::FilePath(glob) => {
                    self.matches_glob(glob, &context.file_path.to_string_lossy())
                }
                DetectorPattern::FileName(name) => {
                    context.file_path.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n == name)
                        .unwrap_or(false)
                }
                DetectorPattern::Attribute(attr) => {
                    context.ast_node.as_ref()
                        .map(|node| node.attributes.contains(attr))
                        .unwrap_or(false)
                }
                DetectorPattern::FunctionName(pattern) => {
                    context.ast_node.as_ref()
                        .and_then(|n| n.name.as_ref())
                        .map(|name| self.matches_pattern(pattern, name))
                        .unwrap_or(false)
                }
                DetectorPattern::ModuleName(pattern) => {
                    context.module_path.last()
                        .map(|m| self.matches_pattern(pattern, m))
                        .unwrap_or(false)
                }
                DetectorPattern::MacroInvocation(macro_name) => {
                    context.source_snippet.contains(macro_name)
                }
                DetectorPattern::Custom(lambda) => {
                    lambda(context)
                }
            }
        })
    }
    
    fn matches_glob(&self, pattern: &str, path: &str) -> bool {
        // Simplified glob matching (real implementation would use glob crate)
        if pattern.contains("**") {
            let parts: Vec<_> = pattern.split("**").collect();
            parts.len() == 2 && path.starts_with(parts[0]) && path.ends_with(parts[1])
        } else if pattern.contains('*') {
            let parts: Vec<_> = pattern.split('*').collect();
            parts.len() == 2 && path.starts_with(parts[0]) && path.ends_with(parts[1])
        } else {
            path == pattern
        }
    }
    
    fn matches_pattern(&self, pattern: &str, text: &str) -> bool {
        if pattern.starts_with('^') && pattern.ends_with('$') {
            // Regex pattern (simplified)
            text == &pattern[1..pattern.len()-1]
        } else if pattern.contains('*') {
            // Wildcard pattern
            let parts: Vec<_> = pattern.split('*').collect();
            parts.len() == 2 && text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            // Exact match
            text == pattern
        }
    }
}

// ============================================================================
// Command Building System
// ============================================================================

/// Runnable types that can be detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnableKind {
    Test,
    Benchmark,
    Example,
    Binary,
    DocTest,
    Script,
    Custom(String),
}

/// Command template with flexible field resolution
#[derive(Debug, Clone)]
pub struct CommandTemplate {
    pub command: CommandField,
    pub channel: Option<CommandField>,
    pub subcommand: Option<CommandField>,
    pub args: Vec<CommandField>,
    pub extra_args: Vec<CommandField>,
    pub extra_test_binary_args: Vec<CommandField>,
    pub extra_env: HashMap<String, CommandField>,
    
    /// Template pattern for command construction
    /// Default: "{extra_env} {command} {channel} {subcommand} {args} {extra_args} -- {extra_test_binary_args}"
    pub template_pattern: String,
    
    /// Custom builder function for complex commands
    pub custom_builder: Option<Arc<dyn Fn(&CommandContext) -> String + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum CommandField {
    /// Static value
    Static(String),
    
    /// Dynamic value resolved by lambda
    Dynamic(Arc<dyn Fn(&CommandContext) -> Option<String> + Send + Sync>),
    
    /// Conditional value
    Conditional {
        condition: Arc<dyn Fn(&CommandContext) -> bool + Send + Sync>,
        then_value: Box<CommandField>,
        else_value: Option<Box<CommandField>>,
    },
}

/// Context for command building
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub file_path: PathBuf,
    pub module_path: Vec<String>,
    pub symbol_name: String,
    pub runnable_kind: RunnableKind,
    pub scope: Scope,
    pub metadata: HashMap<String, String>,
    pub config_overrides: HashMap<String, String>,
}

/// Command builder that resolves templates into executable commands
pub struct CommandBuilder {
    templates: HashMap<String, CommandTemplate>,
    default_template: CommandTemplate,
}

impl CommandBuilder {
    pub fn new(default_template: CommandTemplate) -> Self {
        Self {
            templates: HashMap::new(),
            default_template,
        }
    }
    
    pub fn register_template(&mut self, framework: String, template: CommandTemplate) {
        self.templates.insert(framework, template);
    }
    
    pub fn build_command(&self, context: &CommandContext, framework: Option<&str>) -> String {
        let template = framework
            .and_then(|f| self.templates.get(f))
            .unwrap_or(&self.default_template);
        
        // Use custom builder if provided
        if let Some(ref builder) = template.custom_builder {
            return builder(context);
        }
        
        // Build command from template pattern
        let mut result = template.template_pattern.clone();
        
        // Replace placeholders
        result = result.replace("{command}", &self.resolve_field(&template.command, context));
        
        if let Some(ref channel) = template.channel {
            result = result.replace("{channel}", &self.resolve_field(channel, context));
        } else {
            result = result.replace("{channel}", "");
        }
        
        if let Some(ref subcommand) = template.subcommand {
            result = result.replace("{subcommand}", &self.resolve_field(subcommand, context));
        } else {
            result = result.replace("{subcommand}", "");
        }
        
        let args: Vec<_> = template.args.iter()
            .filter_map(|f| {
                let value = self.resolve_field(f, context);
                if value.is_empty() { None } else { Some(value) }
            })
            .collect();
        result = result.replace("{args}", &args.join(" "));
        
        let extra_args: Vec<_> = template.extra_args.iter()
            .filter_map(|f| {
                let value = self.resolve_field(f, context);
                if value.is_empty() { None } else { Some(value) }
            })
            .collect();
        result = result.replace("{extra_args}", &extra_args.join(" "));
        
        let extra_test_args: Vec<_> = template.extra_test_binary_args.iter()
            .filter_map(|f| {
                let value = self.resolve_field(f, context);
                if value.is_empty() { None } else { Some(value) }
            })
            .collect();
        result = result.replace("{extra_test_binary_args}", &extra_test_args.join(" "));
        
        // Build environment variables
        let env_vars: Vec<_> = template.extra_env.iter()
            .filter_map(|(key, field)| {
                let value = self.resolve_field(field, context);
                if value.is_empty() { None } else { Some(format!("{}={}", key, value)) }
            })
            .collect();
        result = result.replace("{extra_env}", &env_vars.join(" "));
        
        // Clean up multiple spaces
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    
    fn resolve_field(&self, field: &CommandField, context: &CommandContext) -> String {
        match field {
            CommandField::Static(value) => value.clone(),
            CommandField::Dynamic(lambda) => lambda(context).unwrap_or_default(),
            CommandField::Conditional { condition, then_value, else_value } => {
                if condition(context) {
                    self.resolve_field(then_value, context)
                } else if let Some(else_val) = else_value {
                    self.resolve_field(else_val, context)
                } else {
                    String::new()
                }
            }
        }
    }
}

// ============================================================================
// Module Resolution System
// ============================================================================

/// Module resolver trait for different languages
pub trait ModuleResolver: Send + Sync {
    /// Resolve module path from file path and scope
    fn resolve_module_path(
        &self,
        file_path: &Path,
        scope: &Scope,
        project_root: &Path,
    ) -> Vec<String>;
    
    /// Get full qualified name for a symbol
    fn get_qualified_name(
        &self,
        module_path: &[String],
        symbol_name: &str,
    ) -> String;
}

/// Example Rust module resolver
pub struct RustModuleResolver;

impl ModuleResolver for RustModuleResolver {
    fn resolve_module_path(
        &self,
        file_path: &Path,
        _scope: &Scope,
        project_root: &Path,
    ) -> Vec<String> {
        let relative = file_path.strip_prefix(project_root)
            .unwrap_or(file_path);
        
        let mut modules = Vec::new();
        
        // Handle src/ directory
        if let Ok(path) = relative.strip_prefix("src/") {
            // Convert path to module segments
            for component in path.components() {
                if let std::path::Component::Normal(name) = component {
                    let name_str = name.to_string_lossy();
                    if name_str != "mod.rs" && name_str != "lib.rs" && name_str != "main.rs" {
                        let module_name = name_str.trim_end_matches(".rs")
                            .replace('-', "_");
                        modules.push(module_name);
                    }
                }
            }
        }
        
        modules
    }
    
    fn get_qualified_name(
        &self,
        module_path: &[String],
        symbol_name: &str,
    ) -> String {
        if module_path.is_empty() {
            symbol_name.to_string()
        } else {
            format!("{}::{}", module_path.join("::"), symbol_name)
        }
    }
}

// ============================================================================
// Configuration Override System
// ============================================================================

/// Identity for matching configuration overrides
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RunnableIdentity {
    pub file_path: Option<String>,
    pub module: Option<String>,
    pub name: Option<String>,
    pub kind: Option<RunnableKind>,
}

impl RunnableIdentity {
    pub fn matches(&self, other: &RunnableIdentity) -> bool {
        (self.file_path.is_none() || self.file_path == other.file_path) &&
        (self.module.is_none() || self.module == other.module) &&
        (self.name.is_none() || self.name == other.name) &&
        (self.kind.is_none() || self.kind == other.kind)
    }
}

/// Configuration override for specific runnables
#[derive(Debug, Clone)]
pub struct ConfigOverride {
    pub identity: RunnableIdentity,
    pub command: Option<String>,
    pub subcommand: Option<String>,
    pub channel: Option<String>,
    pub extra_args: Vec<String>,
    pub extra_test_binary_args: Vec<String>,
    pub extra_env: HashMap<String, String>,
    pub priority: i32,
}

/// Manages configuration overrides
pub struct ConfigManager {
    overrides: Vec<ConfigOverride>,
}

impl ConfigManager {
    pub fn new() -> Self {
        Self {
            overrides: Vec::new(),
        }
    }
    
    pub fn add_override(&mut self, override_config: ConfigOverride) {
        self.overrides.push(override_config);
        // Sort by priority (higher priority first)
        self.overrides.sort_by_key(|o| std::cmp::Reverse(o.priority));
    }
    
    pub fn get_overrides(&self, identity: &RunnableIdentity) -> Vec<&ConfigOverride> {
        self.overrides
            .iter()
            .filter(|o| o.identity.matches(identity))
            .collect()
    }
    
    pub fn apply_overrides(&self, context: &mut CommandContext, identity: &RunnableIdentity) {
        for override_config in self.get_overrides(identity) {
            if let Some(ref cmd) = override_config.command {
                context.config_overrides.insert("command".to_string(), cmd.clone());
            }
            if let Some(ref subcmd) = override_config.subcommand {
                context.config_overrides.insert("subcommand".to_string(), subcmd.clone());
            }
            if let Some(ref channel) = override_config.channel {
                context.config_overrides.insert("channel".to_string(), channel.clone());
            }
            
            // Merge environment variables
            for (key, value) in &override_config.extra_env {
                context.config_overrides.insert(format!("env.{}", key), value.clone());
            }
        }
    }
}

// ============================================================================
// Plugin Extension API
// ============================================================================

/// Main extension trait that plugins implement
pub trait PluginExtension: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata;
    
    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<String>;
    
    /// Create detectors for this language
    fn create_detectors(&self) -> Vec<Detector>;
    
    /// Create command templates
    fn create_command_templates(&self) -> HashMap<String, CommandTemplate>;
    
    /// Get module resolver
    fn get_module_resolver(&self) -> Box<dyn ModuleResolver>;
    
    /// Parse file and extract scopes
    fn extract_scopes(&self, source: &str, file_path: &Path) -> Vec<Scope>;
    
    /// Custom scope configuration
    fn get_scope_config(&self) -> ScopeConfig {
        ScopeConfig::default()
    }
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub language: String,
    pub description: String,
}

// ============================================================================
// Example: Rust Plugin Implementation
// ============================================================================

pub struct RustPlugin {
    scope_config: ScopeConfig,
}

impl RustPlugin {
    pub fn new() -> Self {
        let mut scope_config = ScopeConfig::default();
        // Rust-specific priority: Function > Method > Impl > Module > File
        scope_config.priority_order = Some(vec![
            ScopeKind::Function,
            ScopeKind::Method,
            ScopeKind::Implementation,
            ScopeKind::Module,
            ScopeKind::File,
        ]);
        
        Self { scope_config }
    }
    
    fn create_cargo_template() -> CommandTemplate {
        CommandTemplate {
            command: CommandField::Static("cargo".to_string()),
            channel: Some(CommandField::Dynamic(Arc::new(|ctx| {
                ctx.config_overrides.get("channel").cloned()
            }))),
            subcommand: Some(CommandField::Dynamic(Arc::new(|ctx| {
                match ctx.runnable_kind {
                    RunnableKind::Test => Some("test".to_string()),
                    RunnableKind::Benchmark => Some("bench".to_string()),
                    RunnableKind::Example => Some("run".to_string()),
                    RunnableKind::Binary => Some("run".to_string()),
                    _ => None,
                }
            }))),
            args: vec![
                CommandField::Conditional {
                    condition: Arc::new(|ctx| ctx.runnable_kind == RunnableKind::Example),
                    then_value: Box::new(CommandField::Static("--example".to_string())),
                    else_value: None,
                },
                CommandField::Dynamic(Arc::new(|ctx| {
                    if ctx.runnable_kind == RunnableKind::Example {
                        Some(ctx.symbol_name.clone())
                    } else {
                        None
                    }
                })),
            ],
            extra_args: vec![],
            extra_test_binary_args: vec![
                CommandField::Dynamic(Arc::new(|ctx| {
                    if ctx.runnable_kind == RunnableKind::Test {
                        Some(format!("{}::{}", ctx.module_path.join("::"), ctx.symbol_name))
                    } else {
                        None
                    }
                })),
                CommandField::Static("--exact".to_string()),
            ],
            extra_env: HashMap::new(),
            template_pattern: "{extra_env} {command} {channel} {subcommand} {args} {extra_args} -- {extra_test_binary_args}".to_string(),
            custom_builder: None,
        }
    }
    
    fn create_rustc_template() -> CommandTemplate {
        CommandTemplate {
            command: CommandField::Static("rustc".to_string()),
            channel: Some(CommandField::Static("+nightly".to_string())),
            subcommand: None,
            args: vec![
                CommandField::Static("--crate-type".to_string()),
                CommandField::Dynamic(Arc::new(|ctx| {
                    match ctx.runnable_kind {
                        RunnableKind::Binary => Some("bin".to_string()),
                        RunnableKind::Test => Some("bin".to_string()),
                        _ => Some("lib".to_string()),
                    }
                })),
                CommandField::Static("--crate-name".to_string()),
                CommandField::Dynamic(Arc::new(|ctx| {
                    Some(ctx.file_path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("main")
                        .to_string())
                })),
                CommandField::Dynamic(Arc::new(|ctx| {
                    Some(ctx.file_path.to_string_lossy().to_string())
                })),
                CommandField::Static("-Zscript".to_string()),
            ],
            extra_args: vec![
                CommandField::Static("-o".to_string()),
                CommandField::Dynamic(Arc::new(|ctx| {
                    Some(ctx.file_path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output")
                        .to_string())
                })),
            ],
            extra_test_binary_args: vec![],
            extra_env: HashMap::new(),
            template_pattern: "{extra_env} {command} {channel} {args} {extra_args}".to_string(),
            custom_builder: Some(Arc::new(|ctx| {
                let filename = ctx.file_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                format!(
                    "rustc +nightly --crate-type bin --crate-name {} {} -Zscript -o {} && ./{}",
                    filename,
                    ctx.file_path.display(),
                    filename,
                    filename
                )
            })),
        }
    }
}

impl PluginExtension for RustPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "rust".to_string(),
            version: "1.0.0".to_string(),
            language: "Rust".to_string(),
            description: "Rust language support with cargo, rustc, and bazel".to_string(),
        }
    }
    
    fn supported_extensions(&self) -> Vec<String> {
        vec!["rs".to_string()]
    }
    
    fn create_detectors(&self) -> Vec<Detector> {
        vec![
            Detector {
                id: "rust_test".to_string(),
                patterns: vec![
                    DetectorPattern::Attribute("#[test]".to_string()),
                ],
                runnable_kind: RunnableKind::Test,
                framework: Some("cargo".to_string()),
                priority: 10,
            },
            Detector {
                id: "rust_bench".to_string(),
                patterns: vec![
                    DetectorPattern::Attribute("#[bench]".to_string()),
                ],
                runnable_kind: RunnableKind::Benchmark,
                framework: Some("cargo".to_string()),
                priority: 10,
            },
            Detector {
                id: "rust_main".to_string(),
                patterns: vec![
                    DetectorPattern::FunctionName("main".to_string()),
                    DetectorPattern::FilePath("src/main.rs".to_string()),
                ],
                runnable_kind: RunnableKind::Binary,
                framework: Some("cargo".to_string()),
                priority: 5,
            },
            Detector {
                id: "rust_example".to_string(),
                patterns: vec![
                    DetectorPattern::FilePath("examples/**/*.rs".to_string()),
                ],
                runnable_kind: RunnableKind::Example,
                framework: Some("cargo".to_string()),
                priority: 8,
            },
            Detector {
                id: "rust_criterion".to_string(),
                patterns: vec![
                    DetectorPattern::MacroInvocation("criterion_main!".to_string()),
                ],
                runnable_kind: RunnableKind::Benchmark,
                framework: Some("criterion".to_string()),
                priority: 12,
            },
            Detector {
                id: "rust_doctest".to_string(),
                patterns: vec![
                    DetectorPattern::Custom(Arc::new(|ctx| {
                        ctx.source_snippet.contains("```rust") || 
                        ctx.source_snippet.contains("```no_run") ||
                        ctx.source_snippet.contains("```should_panic")
                    })),
                ],
                runnable_kind: RunnableKind::DocTest,
                framework: Some("cargo".to_string()),
                priority: 7,
            },
        ]
    }
    
    fn create_command_templates(&self) -> HashMap<String, CommandTemplate> {
        let mut templates = HashMap::new();
        templates.insert("cargo".to_string(), Self::create_cargo_template());
        templates.insert("rustc".to_string(), Self::create_rustc_template());
        templates
    }
    
    fn get_module_resolver(&self) -> Box<dyn ModuleResolver> {
        Box::new(RustModuleResolver)
    }
    
    fn extract_scopes(&self, source: &str, _file_path: &Path) -> Vec<Scope> {
        // This would use tree-sitter-rust to extract actual scopes
        // For demo, returning mock scopes
        vec![
            Scope {
                kind: ScopeKind::File,
                range: 0..source.len(),
                line_range: 0..source.lines().count() as u32,
                name: None,
                metadata: HashMap::new(),
            },
        ]
    }
    
    fn get_scope_config(&self) -> ScopeConfig {
        self.scope_config.clone()
    }
}