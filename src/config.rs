// RUST LEARNING: More specific imports compared to TypeScript
// - `serde` is like JSON.stringify/parse but for any data format
// - `std::` is Rust's standard library (like Node.js built-ins)
// - `thiserror::Error` is for defining custom error types
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex}; // RUST LEARNING: For thread-safe shared state
use thiserror::Error;

// RUST LEARNING: Custom error types using thiserror
// - `#[derive(Error)]` auto-implements the Error trait
// - Much better than just throwing strings like in JavaScript
// - `{0}` in error messages refers to the first field in the variant
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    // RUST LEARNING: `#[from]` automatically converts std::io::Error to ConfigError::Io
    // - Like automatic error wrapping/unwrapping
    // - The `?` operator can convert between compatible error types
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Config directory not found")]
    ConfigDirNotFound, // This variant has no data, like a simple enum value
}

// RUST LEARNING: Type alias to reduce repetition
// - Instead of writing `std::result::Result<T, ConfigError>` everywhere
// - We can just write `Result<T>` - like creating a type alias in TypeScript
// - This creates a module-specific Result type
pub type Result<T> = std::result::Result<T, ConfigError>;

// RUST LEARNING: Generic struct (like TypeScript generics)
// - `<T>` means this struct works with any type T
// - Like `class FileConfig<T>` in TypeScript
pub struct FileConfig<T> {
    file_path: PathBuf, // RUST LEARNING: No `pub` means private field
    // RUST LEARNING: Complex type for thread-safe caching
    // - `Arc<Mutex<Option<T>>>` = thread-safe reference-counted mutex-protected optional value
    // - Arc = like Rc but for multiple threads (Atomically Reference Counted)
    // - Mutex = locks data for thread-safe access
    // - Option<T> = nullable value
    cache: Arc<Mutex<Option<T>>>,
}

// RUST LEARNING: `impl` block defines methods (like class methods in TS)
// - `impl<T>` means implementing for generic type T
impl<T> FileConfig<T>
// RUST LEARNING: `where` clause specifies trait bounds (like interface constraints)
// - `T` must implement these traits to use these methods
// - `for<'de> Deserialize<'de>` handles lifetimes for serde deserialization
// - Like saying "T must be JSON-serializable, have a default value, and be clonable"
where
    T: for<'de> Deserialize<'de> + Serialize + Default + Clone,
{
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            cache: Arc::new(Mutex::new(None)),
        }
    }

    fn load(&self) -> Result<T> {
        debug!("Loading config from: {}", self.file_path.display());
        if let Ok(contents) = fs::read_to_string(&self.file_path) {
            let config = serde_json::from_str(&contents)?;
            debug!("Config loaded successfully");
            Ok(config)
        } else {
            debug!("Config file not found, using default");
            Ok(T::default())
        }
    }

    fn save(&self, data: &T) -> Result<()> {
        debug!("Updating config at: {}", self.file_path.display());
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(data)?;
        fs::write(&self.file_path, contents)?;
        debug!("Config saved successfully");
        Ok(())
    }

    pub fn get_config(&self) -> Result<T> {
        let mut cache = self.cache.lock().unwrap();
        if cache.is_none() {
            *cache = Some(self.load()?);
        }
        Ok(cache.as_ref().unwrap().clone())
    }

    // RUST LEARNING: Method with closure parameter
    // - `<F>` makes this method generic over the closure type F
    // - `F: FnOnce(&mut T)` means F is a closure that takes a mutable reference to T
    // - Like passing a callback function: `updateConfig((config) => { config.foo = 'bar' })`
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut T), // FnOnce = closure that can be called once
    {
        // RUST LEARNING: Mutex locking and dereferencing
        // - `.lock().unwrap()` acquires the mutex lock (like await mutex.acquire())
        // - `mut cache` gets a mutable reference to the Option<T> inside the Mutex
        let mut cache = self.cache.lock().unwrap();
        if cache.is_none() {
            // RUST LEARNING: `*cache = ...` dereferences the mutex guard to assign
            *cache = Some(self.load()?);
        }

        // RUST LEARNING: Clone the data to modify it outside the mutex
        // - Can't modify data while holding the mutex lock
        let mut config = cache.as_ref().unwrap().clone();
        updater(&mut config); // Call the closure with mutable reference

        self.save(&config)?;
        *cache = Some(config); // Update the cache with the modified config
        Ok(())
    }
}

// RUST LEARNING: Struct with serde attributes for JSON serialization
// - `#[derive(...)]` automatically implements common traits
// - Debug = enables {:?} formatting, Clone = makes .clone() work
// - Serialize/Deserialize = JSON conversion, Default = empty/zero values
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    pub args: HashMap<String, serde_json::Value>,
    // RUST LEARNING: `#[serde(rename = "...")]` changes JSON field names
    // - Rust uses snake_case, but JSON often uses camelCase
    // - Like @JsonProperty in Java or @SerializedName in other languages
    #[serde(rename = "scriptDirs")]
    pub script_dirs: Vec<String>, // Vec<T> is like Array<T> in TypeScript
    #[serde(rename = "lastChecked")]
    pub last_checked: Option<u64>, // u64 = unsigned 64-bit integer (like number in TS)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub selected: Vec<String>,
    pub opts: HashMap<String, serde_json::Value>,
}

pub struct Config {
    pub global: FileConfig<GlobalConfig>,
    pub app: FileConfig<AppConfig>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or(ConfigError::ConfigDirNotFound)?;

        let global_path = home_dir.join(".vss.json");
        let app_path = std::env::current_dir()?.join(".vss-app.json");

        Ok(Self {
            global: FileConfig::new(global_path),
            app: FileConfig::new(app_path),
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new().expect("Failed to create config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_config_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config_path = temp_dir.path().join("test_config.json");
        let config = FileConfig::<GlobalConfig>::new(config_path);

        // Test initial load (should be default)
        let initial = config.get_config()?;
        assert!(initial.script_dirs.is_empty());

        // Test update
        config.update_config(|cfg| {
            cfg.script_dirs.push("/test/path".to_string());
            cfg.last_checked = Some(12345);
        })?;

        // Verify update
        let updated = config.get_config()?;
        assert_eq!(updated.script_dirs.len(), 1);
        assert_eq!(updated.script_dirs[0], "/test/path");
        assert_eq!(updated.last_checked, Some(12345));

        Ok(())
    }
}
