use crate::{error::{Error, Result}, types::Runnable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Default)]
pub struct RunnableCache {
    entries: HashMap<PathBuf, CacheEntry>,
    cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    runnables: Vec<Runnable>,
    file_hash: String,
    timestamp: SystemTime,
}

impl RunnableCache {
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self {
            entries: HashMap::new(),
            cache_dir,
        }
    }

    pub fn get(&self, file_path: &Path) -> Option<&Vec<Runnable>> {
        let entry = self.entries.get(file_path)?;

        // Check if file has been modified
        if let Ok(metadata) = std::fs::metadata(file_path) {
            if let Ok(modified) = metadata.modified() {
                if modified > entry.timestamp {
                    return None;
                }
            }
        }

        // Verify file hash
        if let Ok(current_hash) = self.compute_file_hash(file_path) {
            if current_hash != entry.file_hash {
                return None;
            }
        }

        Some(&entry.runnables)
    }

    pub fn insert(&mut self, file_path: PathBuf, runnables: Vec<Runnable>) -> Result<()> {
        let file_hash = self.compute_file_hash(&file_path)?;
        let entry = CacheEntry {
            runnables,
            file_hash,
            timestamp: SystemTime::now(),
        };

        self.entries.insert(file_path.clone(), entry.clone());

        // Persist to disk if cache_dir is set
        if let Some(ref _cache_dir) = self.cache_dir {
            self.save_entry_to_disk(&file_path, &entry)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.entries.clear();

        // Clear disk cache if cache_dir is set
        if let Some(ref cache_dir) = self.cache_dir {
            let _ = std::fs::remove_dir_all(cache_dir);
        }
    }

    pub fn load_from_disk(&mut self) -> Result<()> {
        if let Some(ref cache_dir) = self.cache_dir {
            if !cache_dir.exists() {
                return Ok(());
            }

            for entry in std::fs::read_dir(cache_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&contents) {
                            if let Some(file_name) = path.file_stem() {
                                if let Some(original_path) =
                                    self.decode_cache_filename(file_name.to_string_lossy().as_ref())
                                {
                                    self.entries.insert(original_path, cache_entry);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn save_entry_to_disk(&self, file_path: &Path, entry: &CacheEntry) -> Result<()> {
        if let Some(ref cache_dir) = self.cache_dir {
            std::fs::create_dir_all(cache_dir)?;

            let cache_filename = self.encode_cache_filename(file_path);
            let cache_path = cache_dir.join(format!("{}.json", cache_filename));

            let contents = serde_json::to_string_pretty(entry).map_err(|e| {
                Error::CacheError(format!("Failed to serialize cache entry: {}", e))
            })?;

            std::fs::write(cache_path, contents)?;
        }

        Ok(())
    }

    fn compute_file_hash(&self, file_path: &Path) -> Result<String> {
        if !file_path.exists() {
            // For non-existent files, return a placeholder hash
            return Ok("non-existent".to_string());
        }
        let contents = std::fs::read_to_string(file_path)?;
        let hash = format!("{:x}", md5::compute(contents.as_bytes()));
        Ok(hash)
    }

    fn encode_cache_filename(&self, file_path: &Path) -> String {
        // Simple encoding: replace path separators with double underscores
        file_path
            .to_string_lossy()
            .replace('/', "__")
            .replace('\\', "__")
            .replace(':', "_")
    }

    fn decode_cache_filename(&self, encoded: &str) -> Option<PathBuf> {
        // Simple decoding: replace double underscores with path separators
        let decoded = encoded.replace("__", "/");
        Some(PathBuf::from(decoded))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Position, RunnableKind, Scope, ScopeKind};
    use tempfile::TempDir;

    fn create_test_runnable() -> Runnable {
        Runnable {
            label: "Test".to_string(),
            scope: Scope {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 0,
                },
                kind: ScopeKind::Test,
                name: Some("test".to_string()),
            },
            kind: RunnableKind::Test {
                test_name: "test".to_string(),
                is_async: false,
            },
            module_path: "test".to_string(),
            file_path: PathBuf::from("/test.rs"),
            extended_scope: None,
        }
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = RunnableCache::new(None);
        let file_path = PathBuf::from("/test.rs");
        let runnables = vec![create_test_runnable()];

        // Insert
        cache.insert(file_path.clone(), runnables.clone()).unwrap();

        // Get - returns cached value since we're using a placeholder hash for non-existent files
        assert!(cache.get(&file_path).is_some());

        // Clear
        cache.clear();
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn test_cache_with_disk_persistence() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache_dir = temp_dir.path().to_path_buf();

        let mut cache = RunnableCache::new(Some(cache_dir.clone()));
        let file_path = PathBuf::from("/test.rs");
        let runnables = vec![create_test_runnable()];

        // Insert and save to disk
        cache.insert(file_path.clone(), runnables.clone())?;

        // Create new cache and load from disk
        let mut cache2 = RunnableCache::new(Some(cache_dir));
        cache2.load_from_disk()?;

        // Verify the entry was loaded (though get will fail due to missing file)
        assert!(cache2.entries.contains_key(&file_path));

        Ok(())
    }
}