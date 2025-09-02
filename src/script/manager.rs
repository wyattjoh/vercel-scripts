// RUST LEARNING: `crate::` refers to the current crate's root (like absolute import from src/)
use crate::script::{parser::ScriptParser, types::Script, Result, ScriptError};
use include_dir::{include_dir, Dir};
use log::debug;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// RUST LEARNING: `static` variables are global constants (like const in TS but truly global)
// - `include_dir!()` is a compile-time macro that embeds directory contents in the binary
// - No need for file reading at runtime - everything is baked into the executable
// - `$CARGO_MANIFEST_DIR` = directory containing Cargo.toml
static EMBEDDED_SCRIPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/scripts");

// RUST LEARNING: `include_str!()` embeds file content as a string literal at compile time
static RUNTIME_SCRIPT: &str = include_str!("../runtime/runtime.sh");

pub struct ScriptManager {
    cache_dir: Option<PathBuf>,
}

impl ScriptManager {
    pub fn new() -> Self {
        Self { cache_dir: None }
    }

    fn get_cache_dir(&mut self) -> Result<&PathBuf> {
        if self.cache_dir.is_none() {
            let cache_dir = dirs::cache_dir()
                .or_else(|| dirs::home_dir().map(|h| h.join(".cache")))
                .ok_or_else(|| {
                    ScriptError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Could not find cache directory",
                    ))
                })?
                .join("vercel-scripts");

            fs::create_dir_all(&cache_dir)?;
            self.cache_dir = Some(cache_dir);
        }

        Ok(self.cache_dir.as_ref().unwrap())
    }

    pub fn get_scripts(&mut self, external_dirs: &[String]) -> Result<Vec<Script>> {
        debug!("Starting script discovery and loading");
        let mut all_scripts = Vec::new();

        // Load embedded scripts
        debug!("Loading embedded scripts from binary");
        let embedded_scripts = self.load_embedded_scripts()?;
        debug!("Found {} embedded scripts", embedded_scripts.len());
        all_scripts.extend(embedded_scripts);

        // Load external scripts
        for dir in external_dirs {
            debug!("Loading scripts from directory: {}", dir);
            let external_scripts = self.load_scripts_from_directory(dir, false)?;
            debug!("Found {} scripts in {}", external_scripts.len(), dir);
            all_scripts.extend(external_scripts);
        }

        debug!("Total scripts discovered: {}", all_scripts.len());

        // Sort scripts by dependencies
        let sorted_scripts = self.sort_scripts(all_scripts, external_dirs)?;

        Ok(sorted_scripts)
    }

    pub(crate) fn load_embedded_scripts(&mut self) -> Result<Vec<Script>> {
        let mut scripts = Vec::new();

        // RUST LEARNING: Iterator chain (like JavaScript array methods but lazy/efficient)
        let embedded_scripts: Result<Vec<Script>> = EMBEDDED_SCRIPTS_DIR
            .files()
            // RUST LEARNING: `is_some_and()` is like `?.` chain but for Option
            // - Checks if extension exists AND equals "sh"
            .filter(|file| file.path().extension().is_some_and(|ext| ext == "sh"))
            // RUST LEARNING: `map()` transforms each item (like Array.map in JS)
            .map(|file| {
                // RUST LEARNING: `ok_or_else()` converts Option to Result
                // - If None, calls the closure to create an error
                let content = file.contents_utf8().ok_or_else(|| {
                    ScriptError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Script file is not valid UTF-8",
                    ))
                })?;

                ScriptParser::parse_script(content, file.path(), true)
            })
            // RUST LEARNING: `collect()` consumes the iterator into a collection
            // - Since map() returns Result<Script>, this becomes Result<Vec<Script>>
            // - Automatically handles error propagation (stops on first error)
            .collect();

        scripts.extend(embedded_scripts?);

        Ok(scripts)
    }

    pub(crate) fn load_scripts_from_directory(
        &self,
        dir: &str,
        embedded: bool,
    ) -> Result<Vec<Script>> {
        let mut scripts = Vec::new();
        let dir_path = Path::new(dir);

        if !dir_path.exists() {
            return Ok(scripts);
        }

        // RUST LEARNING: Complex iterator chain with error handling
        let directory_scripts: Result<Vec<Script>> = fs::read_dir(dir_path)?
            // RUST LEARNING: Nested map() - outer handles Result<DirEntry>, inner extracts path
            .map(|entry| entry.map(|e| e.path()))
            // RUST LEARNING: `filter_map()` combines filter + map, removes None values
            .filter_map(|path_result| {
                let path = path_result.ok()?; // Early return None if error
                                              // RUST LEARNING: Method chaining on Option types
                                              // - `and_then()` is like flatMap for Option
                if path.extension().and_then(|s| s.to_str()) == Some("sh") {
                    Some(path)
                } else {
                    None
                }
            })
            .map(|path| {
                let content = fs::read_to_string(&path)?;
                // Canonicalize the path to ensure we have an absolute path
                let absolute_path = path.canonicalize()?;
                ScriptParser::parse_script(&content, &absolute_path, embedded)
            })
            .collect();

        scripts.extend(directory_scripts?);

        Ok(scripts)
    }

    fn sort_scripts(&self, scripts: Vec<Script>, external_dirs: &[String]) -> Result<Vec<Script>> {
        debug!("Building dependency graph for {} scripts", scripts.len());
        let mut graph = DiGraph::new();
        let mut script_indices = HashMap::new();
        let mut path_to_script = HashMap::new();

        // Add all scripts as nodes and create consistent path mappings
        for (i, script) in scripts.iter().enumerate() {
            let node_idx = graph.add_node(i);
            script_indices.insert(i, node_idx);

            // Use consistent path mapping for both embedded and external scripts
            if script.embedded {
                // For embedded scripts, use just the filename as the key
                if let Some(filename) = script.absolute_pathname.file_name() {
                    path_to_script.insert(PathBuf::from(filename), i);
                }
            }

            // Always also store the absolute pathname for lookups
            path_to_script.insert(script.absolute_pathname.clone(), i);
        }

        // Add dependencies as edges
        for script in &scripts {
            if let Some(after_deps) = &script.after {
                for dep in after_deps {
                    debug!(
                        "Processing dependency '{}' for script '{}'",
                        dep, script.name
                    );
                    let dep_script_index =
                        self.resolve_dependency(dep, script, external_dirs, &path_to_script);

                    if let Some(&dep_idx) = dep_script_index {
                        if let (Some(&script_node), Some(&dep_node)) = (
                            script_indices.get(
                                &scripts
                                    .iter()
                                    .position(|s| std::ptr::eq(s, script))
                                    .unwrap(),
                            ),
                            script_indices.get(&dep_idx),
                        ) {
                            debug!(
                                "Adding dependency edge: {} -> {}",
                                scripts[dep_idx].name, script.name
                            );
                            graph.add_edge(dep_node, script_node, ());
                        }
                    } else {
                        // Provide better error message showing which script had the missing dependency
                        return Err(ScriptError::DependencyNotFound(format!(
                            "Dependency '{}' not found in any known script directory for script '{}'",
                            dep, script.name
                        )));
                    }
                }
            }

            // Add required variable dependencies
            if let Some(requirements) = &script.requires {
                for requirement in requirements {
                    debug!(
                        "Processing requirement '{}' for variables {:?} for script '{}'",
                        requirement.script, requirement.variables, script.name
                    );

                    let dep = &requirement.script;
                    let dep_script_index =
                        self.resolve_dependency(dep, script, external_dirs, &path_to_script);

                    if let Some(&dep_idx) = dep_script_index {
                        if let (Some(&script_node), Some(&dep_node)) = (
                            script_indices.get(
                                &scripts
                                    .iter()
                                    .position(|s| std::ptr::eq(s, script))
                                    .unwrap(),
                            ),
                            script_indices.get(&dep_idx),
                        ) {
                            debug!(
                                "Adding requirement edge: {} -> {} for variables {:?}",
                                scripts[dep_idx].name, script.name, requirement.variables
                            );
                            graph.add_edge(dep_node, script_node, ());
                        }
                    } else {
                        // Provide better error message showing which script had the missing requirement
                        return Err(ScriptError::DependencyNotFound(format!(
                            "Required script '{}' not found in any known script directory for script '{}'",
                            dep, script.name
                        )));
                    }
                }
            }
        }

        // Perform topological sort
        debug!("Performing topological sort");
        let sorted_indices = toposort(&graph, None).map_err(|_| ScriptError::CircularDependency)?;

        // Map sorted node indices back to scripts
        let mut sorted_scripts = Vec::new();
        for node_idx in sorted_indices {
            let script_idx = graph[node_idx];
            sorted_scripts.push(scripts[script_idx].clone());
        }

        let script_names: Vec<&str> = sorted_scripts.iter().map(|s| s.name.as_str()).collect();
        debug!("Final execution order: {:?}", script_names);

        Ok(sorted_scripts)
    }

    /// Resolve a dependency to its script index using normalized paths
    fn resolve_dependency<'a>(
        &self,
        dep: &str,
        script: &Script,
        external_dirs: &[String],
        path_to_script: &'a HashMap<std::path::PathBuf, usize>,
    ) -> Option<&'a usize> {
        // First normalize the dependency path (remove leading "./" if present)
        let normalized_dep = ScriptParser::normalize_dependency_path(dep);
        debug!(
            "Resolving dependency '{}' -> '{}' for script '{}'",
            dep, normalized_dep, script.name
        );

        // 1. Try direct filename lookup first (finds embedded scripts by filename)
        let dep_path = std::path::PathBuf::from(&normalized_dep);
        if let Some(script_idx) = path_to_script.get(&dep_path) {
            debug!(
                "Found dependency '{}' via direct filename lookup",
                normalized_dep
            );
            return Some(script_idx);
        }

        // 2. For non-embedded scripts, try resolving relative to script's directory
        if !script.embedded {
            if let Some(script_dir) = script.absolute_pathname.parent() {
                let script_relative_path = script_dir.join(&normalized_dep);
                if let Some(script_idx) = path_to_script.get(&script_relative_path) {
                    debug!(
                        "Found dependency '{}' relative to script directory",
                        normalized_dep
                    );
                    return Some(script_idx);
                }
            }
        }

        // 3. Search in all external directories
        for dir in external_dirs {
            let full_dep_path = std::path::Path::new(dir).join(&normalized_dep);
            if let Some(script_idx) = path_to_script.get(&full_dep_path) {
                debug!(
                    "Found dependency '{}' in external directory '{}'",
                    normalized_dep, dir
                );
                return Some(script_idx);
            }
        }

        debug!(
            "Could not resolve dependency '{}' in any location",
            normalized_dep
        );
        None
    }

    pub fn prepare_runtime(&mut self) -> Result<std::path::PathBuf> {
        let cache_dir = self.get_cache_dir()?;
        debug!("Cache directory: {}", cache_dir.display());
        let runtime_path = cache_dir.join("runtime.sh");

        debug!("Preparing runtime script at: {}", runtime_path.display());
        fs::write(&runtime_path, RUNTIME_SCRIPT)?;
        debug!("Runtime script written ({} bytes)", RUNTIME_SCRIPT.len());

        // RUST LEARNING: Conditional compilation attributes
        // - `#[cfg(unix)]` only compiles this code on Unix-like systems
        // - Like #ifdef in C but more powerful
        // - No runtime check needed - code doesn't exist on non-Unix systems
        #[cfg(unix)]
        {
            // RUST LEARNING: Platform-specific imports inside conditional blocks
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&runtime_path)?.permissions();
            // RUST LEARNING: `0o755` is octal notation (like 0755 in shell)
            perms.set_mode(0o755); // rwxr-xr-x permissions
            fs::set_permissions(&runtime_path, perms)?;
            debug!("Setting executable permissions: 0o755");
        }

        Ok(runtime_path)
    }

    pub fn prepare_script(&mut self, script: &Script, name: &str) -> Result<std::path::PathBuf> {
        let cache_dir = self.get_cache_dir()?;
        // Create a subdirectory with the prefix name
        let script_dir = cache_dir.join(name);

        // Ensure the subdirectory exists
        fs::create_dir_all(&script_dir)?;

        // Extract basename from the original script path
        let basename = script
            .absolute_pathname
            .file_name()
            .ok_or_else(|| ScriptError::InvalidPath(script.absolute_pathname.clone()))?;

        let script_path = script_dir.join(basename);

        debug!(
            "Preparing script {} at: {}",
            script.name,
            script_path.display()
        );

        let content = if script.embedded {
            EMBEDDED_SCRIPTS_DIR
                .get_file(&script.pathname)
                .and_then(|f| f.contents_utf8())
                .ok_or_else(|| {
                    ScriptError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Embedded script not found: {}", script.pathname),
                    ))
                })?
        } else {
            &fs::read_to_string(&script.absolute_pathname)?
        };

        // Check if file exists and has same content
        let needs_write = if script_path.exists() {
            match fs::read_to_string(&script_path) {
                Ok(existing_content) => existing_content != content,
                Err(_) => true, // If we can't read it, we need to write it
            }
        } else {
            true // File doesn't exist, need to write
        };

        if needs_write {
            fs::write(&script_path, content)?;
            debug!("Script content written ({} bytes)", content.len());
        } else {
            debug!("Script content unchanged, skipping write");
        }

        // Check and update executable permissions only if needed
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(&script_path) {
                let current_perms = metadata.permissions();
                let current_mode = current_perms.mode();
                let desired_mode = 0o755;

                if current_mode & 0o777 != desired_mode {
                    let mut perms = current_perms;
                    perms.set_mode(desired_mode);
                    fs::set_permissions(&script_path, perms)?;
                    debug!("Updated permissions to 0o755");
                } else {
                    debug!("Permissions already correct (0o755)");
                }
            }
        }

        Ok(script_path)
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}
