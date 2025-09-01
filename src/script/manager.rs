// RUST LEARNING: `crate::` refers to the current crate's root (like absolute import from src/)
use crate::script::{parser::ScriptParser, types::Script, Result, ScriptError};
use include_dir::{include_dir, Dir}; // RUST LEARNING: For embedding files at compile time
use petgraph::algo::toposort; // Topological sorting algorithm
use petgraph::graph::DiGraph; // RUST LEARNING: Directed graph for dependency resolution
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tempfile::TempDir; // RUST LEARNING: For creating temporary directories

// RUST LEARNING: `static` variables are global constants (like const in TS but truly global)
// - `include_dir!()` is a compile-time macro that embeds directory contents in the binary
// - No need for file reading at runtime - everything is baked into the executable
// - `$CARGO_MANIFEST_DIR` = directory containing Cargo.toml
static EMBEDDED_SCRIPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/scripts");
// RUST LEARNING: `include_str!()` embeds file content as a string literal at compile time
static RUNTIME_SCRIPT: &str = include_str!("../runtime/runtime.sh");

pub struct ScriptManager {
    temp_dir: Option<TempDir>,
}

impl ScriptManager {
    pub fn new() -> Self {
        Self { temp_dir: None }
    }

    pub fn get_scripts(&mut self, external_dirs: &[String]) -> Result<Vec<Script>> {
        let mut all_scripts = Vec::new();

        // Load embedded scripts
        let embedded_scripts = self.load_embedded_scripts()?;
        all_scripts.extend(embedded_scripts);

        // Load external scripts
        for dir in external_dirs {
            let external_scripts = self.load_scripts_from_directory(dir, false)?;
            all_scripts.extend(external_scripts);
        }

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

    fn load_scripts_from_directory(&self, dir: &str, embedded: bool) -> Result<Vec<Script>> {
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
                ScriptParser::parse_script(&content, &path, embedded)
            })
            .collect();

        scripts.extend(directory_scripts?);

        Ok(scripts)
    }

    fn sort_scripts(&self, scripts: Vec<Script>, _external_dirs: &[String]) -> Result<Vec<Script>> {
        let mut graph = DiGraph::new();
        let mut script_indices = HashMap::new();
        let mut path_to_script = HashMap::new();

        // Add all scripts as nodes
        for (i, script) in scripts.iter().enumerate() {
            let node_idx = graph.add_node(i);
            script_indices.insert(i, node_idx);
            path_to_script.insert(script.absolute_pathname.clone(), i);

            // Also add by filename for embedded scripts
            if script.embedded {
                path_to_script.insert(std::path::PathBuf::from(&script.pathname), i);
            }
        }

        // Add dependencies as edges
        for script in &scripts {
            if let Some(after_deps) = &script.after {
                for dep in after_deps {
                    let dep_script_index = if dep.starts_with("./") || dep.starts_with("../") {
                        if script.embedded {
                            // For embedded scripts, relative paths are just filename references
                            let filename = dep.strip_prefix("./").unwrap_or(dep);
                            let dep_path = std::path::PathBuf::from(filename);
                            path_to_script.get(&dep_path)
                        } else {
                            // Relative path - resolve relative to script location
                            let script_dir =
                                script.absolute_pathname.parent().ok_or_else(|| {
                                    ScriptError::DependencyNotFound(format!(
                                        "Cannot resolve parent of {}",
                                        script.absolute_pathname.display()
                                    ))
                                })?;
                            let dep_path = script_dir.join(dep);
                            path_to_script.get(&dep_path)
                        }
                    } else {
                        // Filename lookup in embedded scripts and external dirs
                        let dep_path = std::path::PathBuf::from(dep);
                        path_to_script.get(&dep_path)
                    };

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
                            graph.add_edge(dep_node, script_node, ());
                        }
                    } else {
                        return Err(ScriptError::DependencyNotFound(dep.clone()));
                    }
                }
            }
        }

        // Perform topological sort
        let sorted_indices = toposort(&graph, None).map_err(|_| ScriptError::CircularDependency)?;

        // Map sorted node indices back to scripts
        let mut sorted_scripts = Vec::new();
        for node_idx in sorted_indices {
            let script_idx = graph[node_idx];
            sorted_scripts.push(scripts[script_idx].clone());
        }

        Ok(sorted_scripts)
    }

    pub fn prepare_runtime(&mut self) -> Result<std::path::PathBuf> {
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new()?);
        }

        let temp_dir = self.temp_dir.as_ref().unwrap();
        let runtime_path = temp_dir.path().join("runtime.sh");

        fs::write(&runtime_path, RUNTIME_SCRIPT)?;

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
        }

        Ok(runtime_path)
    }

    pub fn prepare_script(&mut self, script: &Script, name: &str) -> Result<std::path::PathBuf> {
        if self.temp_dir.is_none() {
            self.temp_dir = Some(TempDir::new()?);
        }

        let temp_dir = self.temp_dir.as_ref().unwrap();
        let script_path = temp_dir.path().join(format!("{}.sh", name));

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

        fs::write(&script_path, content)?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        Ok(script_path)
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}
