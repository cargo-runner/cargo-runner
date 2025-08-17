//! Polyrun Host Implementation
//! 
//! This is the native Rust host that:
//! - Owns tree-sitter for scope extraction
//! - Loads WASM plugins via wasmtime
//! - Provides safe, capability-based APIs to plugins
//! - Manages configuration layering

use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use std::ops::Range;
use anyhow::{Result, Context};
use wasmtime::{Engine, Store};
use wasmtime::component::{Component, Linker, bindgen};

// Generate bindings from WIT
bindgen!({
    path: "../polyrun.wit",
    world: "extension",
    async: false,
});

// Re-export types from generated bindings
use self::polyrun::runner::types::{
    ScopeKind, Identity, Scope, Detector, Kv, RunnableTemplate,
    InvocationContext, Resolved,
};

// ============================================================================
// Host State and Core Types
// ============================================================================

/// Main host state that plugins interact with
pub struct HostState {
    /// All scopes extracted via tree-sitter
    scopes: HashMap<u64, ScopeInfo>,
    
    /// Symbol attributes (e.g., #[test])
    attributes: HashMap<u64, Vec<String>>,
    
    /// Scope hierarchy (child -> parent)
    scope_parents: HashMap<u64, u64>,
    
    /// Configuration layers
    config: LayeredConfig,
    
    /// Glob matcher cache
    glob_cache: globset::GlobSet,
    
    /// Next scope ID
    next_scope_id: u64,
}

struct ScopeInfo {
    scope: Scope,
    source_range: Range<usize>,
    file_hash: u64,
}

impl HostState {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
            attributes: HashMap::new(),
            scope_parents: HashMap::new(),
            config: LayeredConfig::default(),
            glob_cache: globset::GlobSetBuilder::new().build().unwrap(),
            next_scope_id: 1,
        }
    }
    
    /// Extract scopes from source file using tree-sitter
    pub fn extract_scopes(&mut self, file_path: &Path, source: &str) -> Result<Vec<Scope>> {
        // This would use tree-sitter-rust, tree-sitter-python, etc.
        // For now, simplified implementation
        let mut scopes = Vec::new();
        
        // File scope
        let file_scope = Scope {
            id: self.next_scope_id,
            kind: ScopeKind::File,
            start_line: 0,
            end_line: source.lines().count() as u32,
            identity: Identity {
                file_path: file_path.to_string_lossy().to_string(),
                module: None,
                name: None,
            },
        };
        
        self.scopes.insert(
            file_scope.id,
            ScopeInfo {
                scope: file_scope.clone(),
                source_range: 0..source.len(),
                file_hash: hash_str(source),
            },
        );
        
        scopes.push(file_scope);
        self.next_scope_id += 1;
        
        // Parse functions, methods, modules, etc. with tree-sitter
        // ... (implementation would go here)
        
        Ok(scopes)
    }
    
    /// Find the best scope for a given line number
    pub fn best_scope(&self, line: u32, file_path: &Path) -> Option<&Scope> {
        let priority = self.config.scope_priority();
        
        let mut candidates: Vec<_> = self.scopes
            .values()
            .filter(|s| {
                s.scope.identity.file_path == file_path.to_string_lossy() &&
                s.scope.start_line <= line && 
                line < s.scope.end_line
            })
            .map(|s| &s.scope)
            .collect();
        
        // Sort by priority, then by range size, then by nesting depth
        candidates.sort_by_key(|s| {
            let priority_index = priority
                .iter()
                .position(|&k| k == s.kind)
                .unwrap_or(usize::MAX);
            
            let range_size = s.end_line - s.start_line;
            let nesting_depth = u32::MAX - s.start_line; // deeper = later start
            
            (priority_index, range_size, nesting_depth)
        });
        
        candidates.first().copied()
    }
}

// ============================================================================
// Host Interface Implementation
// ============================================================================

impl polyrun::runner::host::Host for HostState {
    fn match_glob(&mut self, path: String, patterns: Vec<String>) -> Result<bool> {
        // Build glob set from patterns
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in patterns {
            builder.add(globset::Glob::new(&pattern)?);
        }
        let globs = builder.build()?;
        
        Ok(globs.is_match(&path))
    }
    
    fn symbol_attributes(&mut self, scope_id: u64) -> Result<Vec<String>> {
        Ok(self.attributes
            .get(&scope_id)
            .cloned()
            .unwrap_or_default())
    }
    
    fn compute_module(&mut self, identity: Identity) -> Result<String> {
        // Default Rust module resolver
        // Convert file path to module path
        let path = PathBuf::from(&identity.file_path);
        
        let module_path = if let Some(path_str) = path.to_str() {
            if path_str.contains("/src/") {
                let after_src = path_str.split("/src/").nth(1).unwrap_or("");
                after_src
                    .trim_end_matches(".rs")
                    .replace('/', "::")
                    .replace('-', "_")
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        if let Some(ref module) = identity.module {
            if module_path.is_empty() {
                Ok(module.clone())
            } else {
                Ok(format!("{}::{}", module_path, module))
            }
        } else {
            Ok(module_path)
        }
    }
    
    fn parent_scope(&mut self, scope_id: u64) -> Result<Option<u64>> {
        Ok(self.scope_parents.get(&scope_id).copied())
    }
    
    fn file_exists(&mut self, path: String) -> Result<bool> {
        Ok(Path::new(&path).exists())
    }
    
    fn file_extension(&mut self, path: String) -> Result<Option<String>> {
        Ok(Path::new(&path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(String::from))
    }
}

// ============================================================================
// Configuration Management
// ============================================================================

#[derive(Default)]
struct LayeredConfig {
    /// Host defaults
    host_defaults: Config,
    
    /// Plugin defaults (per plugin)
    plugin_defaults: HashMap<String, Config>,
    
    /// User overrides
    user_config: Option<Config>,
}

#[derive(Default, Clone)]
struct Config {
    scope_order: Vec<ScopeKind>,
    runnable_overrides: HashMap<String, RunnableOverride>,
}

#[derive(Default, Clone)]
struct RunnableOverride {
    channel: Option<String>,
    extra_args: Vec<String>,
    extra_test_binary_args: Vec<String>,
    extra_env: HashMap<String, String>,
}

impl LayeredConfig {
    fn scope_priority(&self) -> Vec<ScopeKind> {
        // User > Plugin > Host
        if let Some(ref user) = self.user_config {
            if !user.scope_order.is_empty() {
                return user.scope_order.clone();
            }
        }
        
        // Default priority
        vec![
            ScopeKind::Function,
            ScopeKind::Method,
            ScopeKind::Module,
            ScopeKind::File,
        ]
    }
    
    fn get_runnable_override(&self, name: &str, plugin: &str) -> Option<&RunnableOverride> {
        // Check user config first
        if let Some(ref user) = self.user_config {
            if let Some(override_cfg) = user.runnable_overrides.get(name) {
                return Some(override_cfg);
            }
        }
        
        // Then plugin defaults
        if let Some(plugin_cfg) = self.plugin_defaults.get(plugin) {
            if let Some(override_cfg) = plugin_cfg.runnable_overrides.get(name) {
                return Some(override_cfg);
            }
        }
        
        // Finally host defaults
        self.host_defaults.runnable_overrides.get(name)
    }
}

// ============================================================================
// Plugin Loading and Management
// ============================================================================

pub struct LoadedPlugin {
    pub metadata: polyrun::runner::plugin::PluginMetadata,
    pub detectors: Vec<Detector>,
    pub runnables: Vec<RunnableTemplate>,
    pub default_config: Option<String>,
    instance: polyrun::runner::plugin::Plugin,
    store: Store<HostState>,
}

pub struct PluginManager {
    engine: Engine,
    linker: Linker<HostState>,
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::<HostState>::new(&engine);
        
        // Add host functions to linker
        polyrun::runner::host::add_to_linker(&mut linker, |state| state)?;
        
        Ok(Self {
            engine,
            linker,
            plugins: HashMap::new(),
        })
    }
    
    pub fn load_plugin(&mut self, name: String, path: &Path, host_state: HostState) -> Result<()> {
        let component = Component::from_file(&self.engine, path)
            .with_context(|| format!("Failed to load component from {:?}", path))?;
        
        let mut store = Store::new(&self.engine, host_state);
        
        let (plugin, _) = polyrun::runner::plugin::Plugin::instantiate(
            &mut store,
            &component,
            &self.linker,
        )?;
        
        // Get plugin metadata
        let metadata = plugin.metadata(&mut store)?;
        
        // Get detectors and runnables
        let detectors = plugin.list_detectors(&mut store)?;
        let runnables = plugin.list_runnables(&mut store)?;
        let default_config = plugin.default_config(&mut store)?;
        
        // Parse and store plugin default config
        if let Some(config_str) = &default_config {
            // Parse TOML and update host_state.config.plugin_defaults
            // ... (TOML parsing code)
        }
        
        let loaded = LoadedPlugin {
            metadata,
            detectors,
            runnables,
            default_config,
            instance: plugin,
            store,
        };
        
        self.plugins.insert(name, loaded);
        Ok(())
    }
    
    pub fn resolve_auto_args(
        &mut self,
        plugin_name: &str,
        runnable_name: &str,
        context: InvocationContext,
    ) -> Result<Resolved> {
        let plugin = self.plugins.get_mut(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin {} not found", plugin_name))?;
        
        plugin.instance.resolve_auto_args(
            &mut plugin.store,
            runnable_name.to_string(),
            context,
        )
    }
}

// ============================================================================
// Command Building
// ============================================================================

pub struct CommandPlan {
    pub program: String,
    pub env: Vec<(String, String)>,
    pub argv: Vec<String>,
}

pub fn build_command(
    template: &RunnableTemplate,
    resolved: &Resolved,
    override_cfg: Option<&RunnableOverride>,
) -> CommandPlan {
    let mut env = Vec::new();
    let mut argv = Vec::new();
    
    // Merge environment variables
    for kv in &template.extra_env {
        env.push((kv.key.clone(), kv.value.clone()));
    }
    for kv in &resolved.extra_env {
        env.push((kv.key.clone(), kv.value.clone()));
    }
    if let Some(cfg) = override_cfg {
        for (k, v) in &cfg.extra_env {
            env.push((k.clone(), v.clone()));
        }
    }
    
    // Build command from template
    let program = template.command.clone();
    
    // Parse template and substitute values
    let mut cmd_parts = Vec::new();
    
    if let Some(ref channel) = template.channel {
        cmd_parts.push(channel.clone());
    }
    if let Some(ref subcmd) = template.subcommand {
        cmd_parts.push(subcmd.clone());
    }
    
    // Add args
    cmd_parts.extend(template.args.clone());
    
    // Add auto-args from resolved
    for kv in &resolved.auto_args {
        if !kv.key.is_empty() {
            cmd_parts.push(kv.key.clone());
        }
        if !kv.value.is_empty() {
            cmd_parts.push(kv.value.clone());
        }
    }
    
    // Add extra args
    cmd_parts.extend(template.extra_args.clone());
    if let Some(cfg) = override_cfg {
        cmd_parts.extend(cfg.extra_args.clone());
    }
    
    // Handle test binary args (after --)
    if !template.extra_test_binary_args.is_empty() {
        cmd_parts.push("--".to_string());
        cmd_parts.extend(template.extra_test_binary_args.clone());
        if let Some(cfg) = override_cfg {
            cmd_parts.extend(cfg.extra_test_binary_args.clone());
        }
    }
    
    argv = cmd_parts;
    
    CommandPlan {
        program,
        env,
        argv,
    }
}

// ============================================================================
// Detector Engine
// ============================================================================

pub struct DetectorEngine {
    detectors: Vec<(String, Detector)>, // (plugin_name, detector)
}

impl DetectorEngine {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }
    
    pub fn add_detectors(&mut self, plugin_name: String, detectors: Vec<Detector>) {
        for detector in detectors {
            if detector.enabled {
                self.detectors.push((plugin_name.clone(), detector));
            }
        }
        
        // Sort by priority (higher first)
        self.detectors.sort_by_key(|(_, d)| std::cmp::Reverse(d.priority));
    }
    
    pub fn find_runnables(
        &self,
        scope: &Scope,
        host: &mut HostState,
    ) -> Vec<(String, String)> { // (plugin_name, runnable_name)
        let mut runnables = Vec::new();
        
        for (plugin_name, detector) in &self.detectors {
            // Check file pattern
            let file_matches = detector.files.is_empty() || 
                host.match_glob(
                    scope.identity.file_path.clone(),
                    detector.files.clone(),
                ).unwrap_or(false);
            
            if !file_matches {
                continue;
            }
            
            // Check scope kind
            if detector.scope != scope.kind {
                continue;
            }
            
            // Check attributes/macros
            if !detector.macros.is_empty() {
                let attrs = host.symbol_attributes(scope.id).unwrap_or_default();
                let has_macro = detector.macros.iter()
                    .any(|m| attrs.contains(m));
                
                if !has_macro {
                    continue;
                }
            }
            
            runnables.push((plugin_name.clone(), detector.runnable.clone()));
        }
        
        runnables
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn hash_str(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_selection() {
        let mut host = HostState::new();
        
        // Add some test scopes
        let file_scope = Scope {
            id: 1,
            kind: ScopeKind::File,
            start_line: 0,
            end_line: 100,
            identity: Identity {
                file_path: "test.rs".to_string(),
                module: None,
                name: None,
            },
        };
        
        let fn_scope = Scope {
            id: 2,
            kind: ScopeKind::Function,
            start_line: 10,
            end_line: 20,
            identity: Identity {
                file_path: "test.rs".to_string(),
                module: None,
                name: Some("test_foo".to_string()),
            },
        };
        
        host.scopes.insert(1, ScopeInfo {
            scope: file_scope.clone(),
            source_range: 0..1000,
            file_hash: 0,
        });
        
        host.scopes.insert(2, ScopeInfo {
            scope: fn_scope.clone(),
            source_range: 100..200,
            file_hash: 0,
        });
        
        // Test that function scope is selected over file scope
        let best = host.best_scope(15, Path::new("test.rs"));
        assert!(best.is_some());
        assert_eq!(best.unwrap().kind, ScopeKind::Function);
    }
}