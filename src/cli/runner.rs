use crate::cli::prompts::{handle_boolean_option, handle_string_option, handle_worktree_option};
use crate::config::Config;
use crate::error::VssResult;
use crate::script::{parser::ScriptParser, Script, ScriptManager, ScriptOpt};
use colored::{Color, Colorize};
use inquire::{list_option::ListOption, validator::Validation, MultiSelect, Text};
use log::debug;
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use tempfile::NamedTempFile;

/// Available colors for script output, matching TypeScript version
const AVAILABLE_COLORS: &[Color] = &[
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Red,
];

/// Result of processing a line through the export parser
#[derive(Debug, Clone)]
enum ExportLineResult {
    RegularLine(String),
    ExportVariable(String, String),
    ExportMarker,
}

/// Streaming parser for export variables that processes lines in real-time
struct ExportParser {
    in_export_section: bool,
    exports: HashMap<String, String>,
    pre_env_file: Option<String>,
    post_env_file: Option<String>,
}

impl ExportParser {
    fn new() -> Self {
        Self {
            in_export_section: false,
            exports: HashMap::new(),
            pre_env_file: None,
            post_env_file: None,
        }
    }

    fn process_line(&mut self, line: &str) -> ExportLineResult {
        let begin_marker = "### VSS_EXPORTS_BEGIN ###";
        let end_marker = "### VSS_EXPORTS_END ###";

        if line.contains(begin_marker) {
            self.in_export_section = true;
            return ExportLineResult::ExportMarker;
        }

        if line.contains(end_marker) {
            self.in_export_section = false;
            return ExportLineResult::ExportMarker;
        }

        if self.in_export_section {
            let line = line.trim();
            if !line.is_empty() {
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim().to_string();
                    let value = line[eq_pos + 1..].trim().to_string();

                    // Handle file paths for environment diffs
                    if key == "PRE_ENV_FILE" {
                        self.pre_env_file = Some(value);
                        return ExportLineResult::ExportMarker;
                    } else if key == "POST_ENV_FILE" {
                        self.post_env_file = Some(value);
                        return ExportLineResult::ExportMarker;
                    }

                    // Remove quotes if present for regular exports
                    let value = if value.starts_with('"') && value.ends_with('"') && value.len() > 1
                    {
                        value[1..value.len() - 1].to_string()
                    } else {
                        value
                    };

                    return ExportLineResult::ExportVariable(key, value);
                }
            }
            return ExportLineResult::ExportMarker; // Empty line in export section
        }

        ExportLineResult::RegularLine(line.to_string())
    }

    fn add_export(&mut self, key: String, value: String) {
        self.exports.insert(key, value);
    }

    fn get_exports(mut self) -> HashMap<String, String> {
        // If we have file paths, parse the diff
        if let (Some(pre_file), Some(post_file)) =
            (self.pre_env_file.clone(), self.post_env_file.clone())
        {
            if let Ok(exports) = self.parse_env_diff(&pre_file, &post_file) {
                for (key, value) in exports {
                    self.exports.insert(key, value);
                }
            }

            // Clean up the temporary files
            let _ = std::fs::remove_file(pre_file);
            let _ = std::fs::remove_file(post_file);
        }

        self.exports
    }

    fn parse_env_diff(
        &self,
        pre_file: &str,
        post_file: &str,
    ) -> Result<HashMap<String, String>, std::io::Error> {
        use std::collections::HashSet;
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Read pre-execution exports
        let pre_exports: HashSet<String> = BufReader::new(File::open(pre_file)?)
            .lines()
            .map_while(Result::ok)
            .collect();

        // Read post-execution exports and find new ones
        let mut new_exports = HashMap::new();
        for line in BufReader::new(File::open(post_file)?)
            .lines()
            .map_while(Result::ok)
        {
            if !pre_exports.contains(&line) {
                // Parse the export line: "export VAR=value" or "declare -x VAR=value"
                if let Some(_export_eq) = line.find('=') {
                    let full_line = &line;
                    // Handle both "export VAR=" and "declare -x VAR=" formats
                    if let Some(var_start) = full_line.find(' ') {
                        let var_part = &full_line[var_start + 1..];
                        if let Some(eq_pos) = var_part.find('=') {
                            let key = var_part[..eq_pos].trim().to_string();
                            let mut value = var_part[eq_pos + 1..].trim().to_string();

                            // Remove surrounding quotes if present
                            if value.starts_with('"') && value.ends_with('"') && value.len() > 1 {
                                value = value[1..value.len() - 1].to_string();
                            }

                            new_exports.insert(key, value);
                        }
                    }
                }
            }
        }

        Ok(new_exports)
    }
}

pub fn run_scripts(replay: bool, debug: bool, config: &Config) -> VssResult<()> {
    let current_config = config.global.get_config().map_err(anyhow::Error::from)?;
    let app_config = config.app.get_config().map_err(anyhow::Error::from)?;
    let mut script_manager = ScriptManager::new();

    let scripts = script_manager
        .get_scripts(&current_config.script_dirs)
        .map_err(anyhow::Error::from)?;

    if scripts.is_empty() {
        println!("{} No scripts found.", "Warning:".yellow());
        println!();
        println!(
            "  Use {} to add a directory with scripts",
            "vss add-script-dir <directory>".cyan()
        );
        return Ok(());
    }

    debug!("Replay mode: {}", replay);
    let selected_scripts = if replay {
        debug!("Using previously selected scripts from saved configuration");
        // Use previously selected scripts
        // RUST LEARNING: `into_iter()` consumes the Vec and gives ownership of each item
        // - Like for...of in JS but transfers ownership
        // - vs `iter()` which would just borrow each item
        scripts
            .into_iter()
            .filter(|script| app_config.selected.contains(&script.pathname))
            .collect()
    } else {
        debug!("Starting interactive script selection");

        // Convert boolean defaults to indices for inquire
        let default_indices: Vec<usize> = scripts
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                if app_config.selected.contains(&s.pathname) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        // Create a validator to ensure proper script selection
        #[derive(Clone)]
        struct ScriptSelectionValidator {
            scripts: Vec<Script>,
        }

        impl inquire::validator::MultiOptionValidator<Script> for ScriptSelectionValidator {
            fn validate(
                &self,
                selected: &[ListOption<&Script>],
            ) -> Result<Validation, inquire::CustomUserError> {
                // Check if no scripts are selected
                if selected.is_empty() {
                    return Ok(Validation::Invalid(
                        "You must select at least one script to run".into(),
                    ));
                }

                // Get selected scripts directly from the list options
                let selected_scripts: Vec<&Script> = selected
                    .iter()
                    .map(|list_option| list_option.value)
                    .collect();

                // Build a set of selected script pathnames for quick lookup
                let selected_pathnames: std::collections::HashSet<&str> = selected_scripts
                    .iter()
                    .map(|script| script.pathname.as_str())
                    .collect();

                // Create consistent mapping from requirement paths to script pathnames
                let mut requirement_to_pathname: std::collections::HashMap<
                    std::path::PathBuf,
                    String,
                > = std::collections::HashMap::new();
                for script in &self.scripts {
                    // Use consistent path mapping for both embedded and external scripts
                    if script.embedded {
                        // For embedded scripts, use just the filename as the key
                        if let Some(filename) = script.absolute_pathname.file_name() {
                            requirement_to_pathname.insert(
                                std::path::PathBuf::from(filename),
                                script.pathname.clone(),
                            );
                        }
                    }

                    // Always also store the absolute pathname for lookups
                    requirement_to_pathname
                        .insert(script.absolute_pathname.clone(), script.pathname.clone());
                }

                // Check if all required dependencies are selected
                for script in &selected_scripts {
                    if let Some(ref requirements) = script.requires {
                        for requirement in requirements {
                            let required_script = &requirement.script;

                            // Resolve requirement path to actual script pathname using normalized path
                            let normalized_requirement =
                                ScriptParser::normalize_dependency_path(required_script);
                            let requirement_path =
                                std::path::PathBuf::from(&normalized_requirement);

                            let resolved_pathname = if let Some(pathname) =
                                requirement_to_pathname.get(&requirement_path)
                            {
                                pathname
                            } else if !script.embedded {
                                // For non-embedded scripts, also try resolving relative to script's directory
                                if let Some(script_dir) = script.absolute_pathname.parent() {
                                    let script_relative_path =
                                        script_dir.join(&normalized_requirement);
                                    requirement_to_pathname
                                        .get(&script_relative_path)
                                        .unwrap_or(required_script)
                                } else {
                                    required_script
                                }
                            } else {
                                required_script
                            };

                            // Check if the resolved script is in our selection
                            if !selected_pathnames.contains(resolved_pathname.as_str()) {
                                return Ok(Validation::Invalid(
                                    format!(
                                        "Script '{}' requires '{}' to be selected as well",
                                        script.name, required_script
                                    )
                                    .into(),
                                ));
                            }
                        }
                    }
                }

                Ok(Validation::Valid)
            }
        }

        let validator = ScriptSelectionValidator {
            scripts: scripts.clone(),
        };

        // RUST LEARNING: Builder pattern with method chaining (like jQuery or axios)
        let selections = MultiSelect::new("Which scripts do you want to run?", scripts.clone())
            .with_default(&default_indices)
            .with_page_size(scripts.len())
            .with_validator(validator)
            .prompt()?; // The `?` propagates any interaction errors

        // Save selections
        config
            .app
            .update_config(|cfg| {
                cfg.selected = selections.iter().map(|s| s.pathname.clone()).collect();
            })
            .map_err(anyhow::Error::from)?;

        selections
    };

    let script_names: Vec<&str> = selected_scripts.iter().map(|s| s.name.as_str()).collect();
    debug!("Selected scripts: {:?}", script_names);

    if selected_scripts.is_empty() {
        println!("No scripts selected.");
        return Ok(());
    }

    // Collect arguments and options
    let mut global_args = current_config.args.clone();
    let mut app_opts = app_config.opts.clone();

    collect_script_inputs(&selected_scripts, &mut global_args, &mut app_opts)?;

    // Save updated args and opts
    if !global_args.is_empty() {
        config
            .global
            .update_config(|cfg| {
                cfg.args = global_args.clone();
            })
            .map_err(anyhow::Error::from)?;
    }

    if !app_opts.is_empty() {
        config
            .app
            .update_config(|cfg| {
                cfg.opts = app_opts.clone();
            })
            .map_err(anyhow::Error::from)?;
    }

    // Execute scripts
    execute_scripts(
        &selected_scripts,
        &global_args,
        &app_opts,
        &mut script_manager,
        debug,
    )
}

fn collect_script_inputs(
    scripts: &[Script],
    global_args: &mut HashMap<String, serde_json::Value>,
    app_opts: &mut HashMap<String, serde_json::Value>,
) -> VssResult<()> {
    for script in scripts {
        debug!("Collecting arguments for script: {}", script.name);
        // Collect script arguments
        if let Some(ref args) = script.args {
            for arg in args {
                if !global_args.contains_key(&arg.name) {
                    let value: String = Text::new(&format!(
                        "Enter a value for {} - {}",
                        arg.name.cyan(),
                        arg.description
                    ))
                    .with_default(
                        dirs::home_dir()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .as_ref(),
                    )
                    .prompt()?;

                    global_args.insert(arg.name.clone(), serde_json::Value::String(value));
                }
            }
        }

        debug!("Collecting options for script: {}", script.name);
        // Collect script options
        if let Some(ref opts) = script.opts {
            for opt in opts {
                if !app_opts.contains_key(opt.name()) {
                    match opt {
                        ScriptOpt::Boolean { default, .. } => {
                            let value = handle_boolean_option(opt, default)?;
                            app_opts.insert(opt.name().to_string(), serde_json::Value::Bool(value));
                            global_args
                                .insert(opt.name().to_string(), serde_json::Value::Bool(value));
                        }
                        ScriptOpt::String {
                            default,
                            pattern,
                            pattern_help,
                            ..
                        } => {
                            if let Some(value) =
                                handle_string_option(opt, default, pattern, pattern_help)?
                            {
                                app_opts.insert(
                                    opt.name().to_string(),
                                    serde_json::Value::String(value.clone()),
                                );
                                global_args.insert(
                                    opt.name().to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                        }
                        ScriptOpt::Worktree { base_dir_arg, .. } => {
                            if let Some(value) =
                                handle_worktree_option(opt, base_dir_arg, global_args)?
                            {
                                app_opts.insert(
                                    opt.name().to_string(),
                                    serde_json::Value::String(value.clone()),
                                );
                                global_args.insert(
                                    opt.name().to_string(),
                                    serde_json::Value::String(value),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

// RUST LEARNING: Function signature with multiple reference parameters
// - `&[Script]` is a slice (like Array<Script> but borrowed, not owned)
// - `&mut ScriptManager` is a mutable reference (like passing by reference in C++)
// - All the `&` parameters are borrowing, not taking ownership
fn execute_scripts(
    scripts: &[Script],
    global_args: &HashMap<String, serde_json::Value>,
    app_opts: &HashMap<String, serde_json::Value>,
    script_manager: &mut ScriptManager,
    debug: bool,
) -> VssResult<()> {
    // Store exported variables from each script for later use by dependent scripts
    let mut script_exports: HashMap<String, HashMap<String, String>> = HashMap::new();

    // Create consistent mapping from requirement paths to script pathnames for variable lookup
    let mut requirement_to_pathname: HashMap<std::path::PathBuf, String> = HashMap::new();
    for script in scripts.iter() {
        // Use consistent path mapping for both embedded and external scripts
        if script.embedded {
            // For embedded scripts, use just the filename as the key
            if let Some(filename) = script.absolute_pathname.file_name() {
                requirement_to_pathname
                    .insert(std::path::PathBuf::from(filename), script.pathname.clone());
            }
        }

        // Always also store the absolute pathname for lookups
        requirement_to_pathname.insert(script.absolute_pathname.clone(), script.pathname.clone());
    }
    // RUST LEARNING: `enumerate()` gives (index, item) tuples (like Array.entries() in JS)
    for (index, script) in scripts.iter().enumerate() {
        // RUST LEARNING: Modulo operator for cycling through colors (like TypeScript version)
        let color = AVAILABLE_COLORS[index % AVAILABLE_COLORS.len()];

        debug!("Executing script: {}", script.name);

        // RUST LEARNING: Method chaining - format!() creates String, .color() adds color
        println!("{}", format!("✨ Running {}...", script.name).color(color));

        // Prepare environment variables
        let mut env_vars = HashMap::new();

        // Add debug flag if enabled
        if debug {
            env_vars.insert("VSS_DEBUG".to_string(), "1".to_string());
        }

        // Add script arguments
        // RUST LEARNING: `if let Some(ref args)` pattern matches Option and borrows the content
        // - `ref` makes `args` a reference instead of taking ownership
        // - Like: if (script.args) { const args = script.args; } but with borrowing
        if let Some(ref args) = script.args {
            for arg in args {
                if let Some(value) = global_args.get(&arg.name) {
                    // RUST LEARNING: Pattern matching on enum variants to convert JSON values
                    // - Each arm handles different JSON value types
                    // - More type-safe than just calling .toString() in JS
                    let env_value = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => value.to_string(), // Fallback for other types
                    };
                    env_vars.insert(arg.name.clone(), env_value.clone());
                    println!("    {}: {}", arg.name.color(color), env_value);
                }
            }
        }

        // Add script options
        if let Some(ref opts) = script.opts {
            for opt in opts {
                if let Some(value) = app_opts.get(opt.name()) {
                    match value {
                        serde_json::Value::Null => continue, // Skip null values
                        _ => {
                            let env_value = match value {
                                serde_json::Value::String(s) => s.clone(),
                                serde_json::Value::Bool(b) => b.to_string(),
                                serde_json::Value::Number(n) => n.to_string(),
                                _ => value.to_string(),
                            };
                            env_vars.insert(opt.name().to_string(), env_value.clone());
                            println!("    {}: {}", opt.name().color(color), env_value);
                        }
                    }
                }
            }
        }

        // Add required variables from dependencies with validation
        if let Some(ref requirements) = script.requires {
            let mut validation_errors = Vec::new();

            for requirement in requirements {
                // Resolve requirement path to actual script pathname using normalized path
                let normalized_requirement =
                    ScriptParser::normalize_dependency_path(&requirement.script);
                let requirement_path = std::path::PathBuf::from(&normalized_requirement);

                let lookup_key =
                    if let Some(pathname) = requirement_to_pathname.get(&requirement_path) {
                        pathname
                    } else if !script.embedded {
                        // For non-embedded scripts, also try resolving relative to script's directory
                        if let Some(script_dir) = script.absolute_pathname.parent() {
                            let script_relative_path = script_dir.join(&normalized_requirement);
                            requirement_to_pathname
                                .get(&script_relative_path)
                                .unwrap_or(&requirement.script)
                        } else {
                            &requirement.script
                        }
                    } else {
                        &requirement.script
                    };

                if let Some(exported_vars) = script_exports.get(lookup_key) {
                    for var_name in &requirement.variables {
                        if let Some(var_value) = exported_vars.get(var_name) {
                            env_vars.insert(var_name.clone(), var_value.clone());
                            println!(
                                "    {} (from {}): {}",
                                var_name.color(color),
                                requirement.script.color(color),
                                var_value
                            );
                        } else {
                            validation_errors.push(format!(
                                "Variable '{}' required by script '{}' was not exported by script '{}'",
                                var_name, script.name, requirement.script
                            ));
                        }
                    }
                } else {
                    validation_errors.push(format!(
                        "Script '{}' requires variables from '{}', but that script did not export any variables",
                        script.name, requirement.script
                    ));
                }
            }

            // Fail execution if any required variables are missing
            if !validation_errors.is_empty() {
                eprintln!(
                    "{} Script '{}' failed due to missing required variables:",
                    "Error:".red(),
                    script.name
                );
                for error in &validation_errors {
                    eprintln!("  • {}", error);
                }
                eprintln!("\n{}", "Hint: Ensure that required scripts properly export their variables using 'export VARIABLE_NAME=value'".cyan());
                std::process::exit(1);
            }
        }

        debug!("Script env vars: {:?}", env_vars);

        // Prepare runtime and script
        let runtime_path = script_manager
            .prepare_runtime()
            .map_err(anyhow::Error::from)?;
        let script_path = script_manager
            .prepare_script(script, "script")
            .map_err(anyhow::Error::from)?;

        // Create temporary files for export collection
        let pre_env_file = NamedTempFile::new().map_err(anyhow::Error::from)?;
        let post_env_file = NamedTempFile::new().map_err(anyhow::Error::from)?;
        
        // Add temp file paths to environment variables
        env_vars.insert("VSS_PRE_ENV_FILE".to_string(), pre_env_file.path().to_string_lossy().to_string());
        env_vars.insert("VSS_POST_ENV_FILE".to_string(), post_env_file.path().to_string_lossy().to_string());

        // Execute script
        // RUST LEARNING: Option method chaining with `as_deref()`
        // - Converts Option<String> to Option<&str> for comparison
        let stdio = if script.stdin.as_deref() == Some("inherit") {
            Stdio::inherit() // Pass through terminal input/output
        } else {
            Stdio::piped() // Capture output for processing
        };

        debug!(
            "Script command: {} {}",
            runtime_path.display(),
            script_path.display()
        );
        debug!(
            "Script stdio mode: {:?}",
            if script.stdin.as_deref() == Some("inherit") {
                "inherit"
            } else {
                "piped"
            }
        );

        // RUST LEARNING: Builder pattern for process configuration
        // - Each method returns Self, allowing method chaining
        // - `spawn()` starts the process and returns a Child handle
        // Ensure proper shell environment (like TypeScript version's shell: true)
        let inherit_all = script.stdin.as_deref() == Some("inherit");
        let mut cmd = Command::new(&runtime_path)
            .arg(&script_path)
            .stdin(stdio)
            .stdout(if inherit_all { Stdio::inherit() } else { Stdio::piped() })
            .stderr(if inherit_all { Stdio::inherit() } else { Stdio::piped() })
            .envs(&env_vars) // Set all environment variables at once
            .env(
                "SHELL",
                env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()),
            ) // Ensure shell is set
            .spawn()
            .map_err(anyhow::Error::from)?;

        // Handle output streaming with export parsing

        // Use channels to collect exports from the streaming thread
        let (export_tx, _export_rx) = std::sync::mpsc::channel();

        // Store thread handles to ensure they complete
        let mut thread_handles: Vec<JoinHandle<()>> = Vec::new();

        if script.stdin.as_deref() != Some("inherit") {
            debug!("Spawning streaming output handler with export parsing");

            // RUST LEARNING: `take()` moves the value out of the Option, leaving None
            if let Some(stdout) = cmd.stdout.take() {
                let reader = BufReader::new(stdout);
                let script_name = script.pathname.clone();
                let color_clone = color;
                let export_tx_clone = export_tx.clone();

                let stdout_handle = thread::spawn(move || {
                    let mut export_parser = ExportParser::new();

                    for line in reader.lines().map_while(Result::ok) {
                        match export_parser.process_line(&line) {
                            ExportLineResult::RegularLine(content) => {
                                println!(
                                    "{} {}",
                                    format!("[{}]", script_name).color(color_clone),
                                    content
                                );
                                // Flush stdout to ensure immediate output
                                let _ = io::stdout().flush();
                            }
                            ExportLineResult::ExportVariable(key, value) => {
                                export_parser.add_export(key, value);
                            }
                            ExportLineResult::ExportMarker => {
                                // Don't display export markers
                            }
                        }
                    }

                    // Send collected exports back to main thread
                    let _ = export_tx_clone.send(export_parser.get_exports());
                });
                thread_handles.push(stdout_handle);
            }

            if let Some(stderr) = cmd.stderr.take() {
                let reader = BufReader::new(stderr);
                let script_name = script.pathname.clone();
                let color_clone = color;

                let stderr_handle = thread::spawn(move || {
                    for line in reader.lines().map_while(Result::ok) {
                        println!(
                            "{} {}",
                            format!("[{}]", script_name).color(color_clone),
                            line
                        );
                        // Flush stdout to ensure immediate output
                        let _ = io::stdout().flush();
                    }
                });
                thread_handles.push(stderr_handle);
            }
        }

        // Drop the sender so recv() will unblock when all threads finish
        drop(export_tx);

        // Wait for the process to complete
        let exit_status = cmd.wait().map_err(anyhow::Error::from)?;

        // Wait for all output threads to complete before collecting exports and returning
        // This ensures all output is displayed even for fast-completing scripts
        for handle in thread_handles {
            let _ = handle.join(); // Ignore join errors, focus on output completion
        }

        // Collect exports directly from temp files
        let exports = read_exports_from_files(pre_env_file.path(), post_env_file.path());

        // Store exports for dependent scripts
        if !exports.is_empty() {
            debug!("Script '{}' exported variables: {:?}", script.name, exports);
            script_exports.insert(script.pathname.clone(), exports);
        }

        debug!(
            "Script {} completed with exit code: {:?}",
            script.name,
            exit_status.code()
        );

        if !exit_status.success() {
            eprintln!(
                "{} Script {} failed with exit code: {}",
                "Error:".red(),
                script.name,
                exit_status
            );
            std::process::exit(exit_status.code().unwrap_or(1));
        }
    }

    Ok(())
}

fn read_exports_from_files(pre_env_path: &std::path::Path, post_env_path: &std::path::Path) -> HashMap<String, String> {
        use std::collections::HashSet;
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let mut exports = HashMap::new();
        
        // Read pre-execution exports if file exists
        let pre_exports: HashSet<String> = if pre_env_path.exists() {
            match File::open(pre_env_path) {
                Ok(file) => BufReader::new(file)
                    .lines()
                    .map_while(Result::ok)
                    .collect(),
                Err(_) => HashSet::new(),
            }
        } else {
            HashSet::new()
        };

        // Read post-execution exports and find new ones
        if post_env_path.exists() {
            if let Ok(file) = File::open(post_env_path) {
                for line in BufReader::new(file).lines().map_while(Result::ok) {
                    if !pre_exports.contains(&line) {
                        // Parse the export line: "export VAR=value" or "declare -x VAR=value"
                        if let Some(eq_pos) = line.find('=') {
                            // Extract the variable assignment part (everything from the last space before '=' to the end)
                            let before_eq = &line[..eq_pos];
                            if let Some(var_start) = before_eq.rfind(' ') {
                                let key = before_eq[var_start + 1..].trim().to_string();
                                let value = line[eq_pos + 1..].trim();
                                
                                // Remove quotes if present
                                let clean_value = if (value.starts_with('"') && value.ends_with('"') && value.len() > 1) 
                                    || (value.starts_with('\'') && value.ends_with('\'') && value.len() > 1) {
                                    value[1..value.len() - 1].to_string()
                                } else {
                                    value.to_string()
                                };
                                
                                exports.insert(key, clean_value);
                            }
                        }
                    }
                }
            }
        }
        
        exports
}

/// Parse exported variables from script output
/// Looks for content between ### VSS_EXPORTS_BEGIN ### and ### VSS_EXPORTS_END ### markers
#[cfg(test)]
fn parse_exported_variables(output: &str) -> HashMap<String, String> {
    let mut exports = HashMap::new();

    let begin_marker = "### VSS_EXPORTS_BEGIN ###";
    let end_marker = "### VSS_EXPORTS_END ###";

    if let Some(start) = output.find(begin_marker) {
        if let Some(end) = output.find(end_marker) {
            if start < end {
                let exports_section = &output[start + begin_marker.len()..end];

                for line in exports_section.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Some(eq_pos) = line.find('=') {
                        let key = line[..eq_pos].trim().to_string();
                        let value = line[eq_pos + 1..].trim().to_string();

                        // Remove quotes if present
                        let value =
                            if value.starts_with('"') && value.ends_with('"') && value.len() > 1 {
                                value[1..value.len() - 1].to_string()
                            } else {
                                value
                            };

                        exports.insert(key, value);
                    }
                }
            }
        }
    }

    exports
}

/// Remove export markers from output for display purposes
#[cfg(test)]
fn filter_export_markers(output: &str) -> String {
    let begin_marker = "### VSS_EXPORTS_BEGIN ###";
    let end_marker = "### VSS_EXPORTS_END ###";

    let mut result = String::new();
    let mut in_exports_section = false;

    for line in output.lines() {
        if line.contains(begin_marker) {
            in_exports_section = true;
            continue;
        }
        if line.contains(end_marker) {
            in_exports_section = false;
            continue;
        }

        if !in_exports_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Remove trailing newline
    if result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exported_variables() {
        let output = r#"Script output before
### VSS_EXPORTS_BEGIN ###
PROJECT_ID=abc123
API_KEY="secret-key"
DEBUG_MODE=true
### VSS_EXPORTS_END ###
Script output after"#;

        let exports = parse_exported_variables(output);

        assert_eq!(exports.len(), 3);
        assert_eq!(exports.get("PROJECT_ID"), Some(&"abc123".to_string()));
        assert_eq!(exports.get("API_KEY"), Some(&"secret-key".to_string()));
        assert_eq!(exports.get("DEBUG_MODE"), Some(&"true".to_string()));
    }

    #[test]
    fn test_parse_exported_variables_empty() {
        let output = r#"Script output only"#;
        let exports = parse_exported_variables(output);
        assert_eq!(exports.len(), 0);
    }

    #[test]
    fn test_parse_exported_variables_empty_section() {
        let output = r#"Script output before
### VSS_EXPORTS_BEGIN ###
### VSS_EXPORTS_END ###
Script output after"#;

        let exports = parse_exported_variables(output);
        assert_eq!(exports.len(), 0);
    }

    #[test]
    fn test_parse_exported_variables_with_quotes() {
        let output = r#"### VSS_EXPORTS_BEGIN ###
VAR_WITH_QUOTES="value with spaces"
VAR_WITHOUT_QUOTES=simple_value
### VSS_EXPORTS_END ###"#;

        let exports = parse_exported_variables(output);

        assert_eq!(exports.len(), 2);
        assert_eq!(
            exports.get("VAR_WITH_QUOTES"),
            Some(&"value with spaces".to_string())
        );
        assert_eq!(
            exports.get("VAR_WITHOUT_QUOTES"),
            Some(&"simple_value".to_string())
        );
    }

    #[test]
    fn test_filter_export_markers() {
        let output = r#"Line 1
Line 2
### VSS_EXPORTS_BEGIN ###
PROJECT_ID=abc123
API_KEY=secret
### VSS_EXPORTS_END ###
Line 3
Line 4"#;

        let filtered = filter_export_markers(output);
        let expected = "Line 1\nLine 2\nLine 3\nLine 4";

        assert_eq!(filtered, expected);
    }

    #[test]
    fn test_filter_export_markers_no_exports() {
        let output = r#"Line 1
Line 2
Line 3"#;

        let filtered = filter_export_markers(output);
        assert_eq!(filtered, output);
    }

    #[test]
    fn test_export_parser_streaming() {
        let mut parser = ExportParser::new();

        let lines = vec![
            "Regular line 1",
            "### VSS_EXPORTS_BEGIN ###",
            "PROJECT_ID=abc123",
            "API_KEY=\"secret-key\"",
            "### VSS_EXPORTS_END ###",
            "Regular line 2",
        ];

        let mut regular_lines = Vec::new();

        for line in lines {
            match parser.process_line(line) {
                ExportLineResult::RegularLine(content) => {
                    regular_lines.push(content);
                }
                ExportLineResult::ExportVariable(key, value) => {
                    parser.add_export(key, value);
                }
                ExportLineResult::ExportMarker => {
                    // Ignore markers
                }
            }
        }

        let exports = parser.get_exports();

        // Check that regular lines are preserved
        assert_eq!(regular_lines, vec!["Regular line 1", "Regular line 2"]);

        // Check that exports are captured
        assert_eq!(exports.len(), 2);
        assert_eq!(exports.get("PROJECT_ID"), Some(&"abc123".to_string()));
        assert_eq!(exports.get("API_KEY"), Some(&"secret-key".to_string()));
    }

    #[test]
    fn test_export_parser_no_exports() {
        let mut parser = ExportParser::new();

        let lines = vec!["Regular line 1", "Regular line 2"];

        let mut regular_lines = Vec::new();

        for line in lines {
            match parser.process_line(line) {
                ExportLineResult::RegularLine(content) => {
                    regular_lines.push(content);
                }
                ExportLineResult::ExportVariable(key, value) => {
                    parser.add_export(key, value);
                }
                ExportLineResult::ExportMarker => {
                    // Ignore markers
                }
            }
        }

        let exports = parser.get_exports();

        // Check that regular lines are preserved
        assert_eq!(regular_lines, vec!["Regular line 1", "Regular line 2"]);

        // Check that no exports are captured
        assert_eq!(exports.len(), 0);
    }
    
    #[test]
    fn test_read_exports_from_files_basic() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        // Create temp files with test data
        let mut pre_file = NamedTempFile::new().unwrap();
        let mut post_file = NamedTempFile::new().unwrap();
        
        // Pre-execution exports (existing variables)
        writeln!(pre_file, "declare -x HOME=\"/home/user\"").unwrap();
        writeln!(pre_file, "declare -x PATH=\"/usr/bin:/bin\"").unwrap();
        
        // Post-execution exports (existing + new variables)
        writeln!(post_file, "declare -x HOME=\"/home/user\"").unwrap();
        writeln!(post_file, "declare -x PATH=\"/usr/bin:/bin\"").unwrap();
        writeln!(post_file, "declare -x PROJECT_ID=\"abc123\"").unwrap();
        writeln!(post_file, "declare -x API_KEY=\"secret-key\"").unwrap();
        
        let exports = read_exports_from_files(pre_file.path(), post_file.path());
        
        assert_eq!(exports.len(), 2);
        assert_eq!(exports.get("PROJECT_ID"), Some(&"abc123".to_string()));
        assert_eq!(exports.get("API_KEY"), Some(&"secret-key".to_string()));
    }
    
    #[test] 
    fn test_read_exports_from_files_with_quotes() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        let pre_file = NamedTempFile::new().unwrap();
        let mut post_file = NamedTempFile::new().unwrap();
        
        // Only post-execution exports for simpler test
        writeln!(post_file, "declare -x VAR_WITH_DOUBLE_QUOTES=\"value with spaces\"").unwrap();
        writeln!(post_file, "declare -x VAR_WITH_SINGLE_QUOTES='single quoted'").unwrap();
        writeln!(post_file, "declare -x VAR_WITHOUT_QUOTES=simple_value").unwrap();
        
        let exports = read_exports_from_files(pre_file.path(), post_file.path());
        
        assert_eq!(exports.len(), 3);
        assert_eq!(exports.get("VAR_WITH_DOUBLE_QUOTES"), Some(&"value with spaces".to_string()));
        assert_eq!(exports.get("VAR_WITH_SINGLE_QUOTES"), Some(&"single quoted".to_string()));
        assert_eq!(exports.get("VAR_WITHOUT_QUOTES"), Some(&"simple_value".to_string()));
    }
    
    #[test]
    fn test_read_exports_from_files_export_format() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        let pre_file = NamedTempFile::new().unwrap();
        let mut post_file = NamedTempFile::new().unwrap();
        
        // Test both "export VAR=" and "declare -x VAR=" formats
        writeln!(post_file, "export PROJECT_ID=abc123").unwrap();
        writeln!(post_file, "declare -x API_KEY=secret-key").unwrap();
        
        let exports = read_exports_from_files(pre_file.path(), post_file.path());
        
        assert_eq!(exports.len(), 2);
        assert_eq!(exports.get("PROJECT_ID"), Some(&"abc123".to_string()));
        assert_eq!(exports.get("API_KEY"), Some(&"secret-key".to_string()));
    }
    
    #[test]
    fn test_read_exports_from_files_empty() {
        use tempfile::NamedTempFile;
        
        let pre_file = NamedTempFile::new().unwrap();
        let post_file = NamedTempFile::new().unwrap();
        
        // Both files are empty
        let exports = read_exports_from_files(pre_file.path(), post_file.path());
        
        assert_eq!(exports.len(), 0);
    }
    
    #[test]
    fn test_read_exports_from_files_no_new_exports() {
        use tempfile::NamedTempFile;
        use std::io::Write;
        
        let mut pre_file = NamedTempFile::new().unwrap();
        let mut post_file = NamedTempFile::new().unwrap();
        
        // Same exports in both files (no new variables)
        let exports_content = "declare -x HOME=\"/home/user\"\ndeclare -x PATH=\"/usr/bin:/bin\"";
        writeln!(pre_file, "{}", exports_content).unwrap();
        writeln!(post_file, "{}", exports_content).unwrap();
        
        let exports = read_exports_from_files(pre_file.path(), post_file.path());
        
        assert_eq!(exports.len(), 0);
    }
    
    #[test]
    fn test_read_exports_from_files_missing_files() {
        use std::path::Path;
        
        let missing_pre = Path::new("/nonexistent/pre.txt");
        let missing_post = Path::new("/nonexistent/post.txt");
        
        let exports = read_exports_from_files(missing_pre, missing_post);
        
        // Should handle missing files gracefully
        assert_eq!(exports.len(), 0);
    }
}
