use crate::cli::prompts::{handle_boolean_option, handle_string_option, handle_worktree_option};
use crate::config::Config;
use crate::script::{Script, ScriptManager, ScriptOpt};
use colored::Colorize; // RUST LEARNING: Trait to add color methods to strings
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect}; // RUST LEARNING: CLI interaction library
use log::debug;
use std::collections::HashMap;
use std::io::{BufRead, BufReader}; // RUST LEARNING: Buffered I/O for reading process output
use std::process::{Command, Stdio}; // RUST LEARNING: For process execution and I/O redirection
use std::thread; // RUST LEARNING: For spawning threads (like Web Workers or Node workers)

pub fn run_scripts(replay: bool, config: &Config) -> anyhow::Result<()> {
    let current_config = config.global.get_config()?;
    let app_config = config.app.get_config()?;
    let mut script_manager = ScriptManager::new();

    let scripts = script_manager.get_scripts(&current_config.script_dirs)?;

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
        // Interactive script selection
        // RUST LEARNING: Type annotation `Vec<String>` is explicit but often optional
        // - Rust can usually infer types from usage
        let script_names: Vec<String> = scripts.iter().map(|s| s.name.clone()).collect();
        let defaults: Vec<bool> = scripts
            .iter()
            .map(|s| app_config.selected.contains(&s.pathname))
            .collect();

        // RUST LEARNING: Builder pattern with method chaining (like jQuery or axios)
        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Which scripts do you want to run?")
            .items(&script_names)
            .defaults(&defaults)
            .interact()?; // The `?` propagates any interaction errors

        let selected: Vec<Script> = selections
            .into_iter()
            .map(|idx| scripts[idx].clone())
            .collect();

        // Save selections
        config.app.update_config(|cfg| {
            cfg.selected = selected.iter().map(|s| s.pathname.clone()).collect();
        })?;

        selected
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
        config.global.update_config(|cfg| {
            cfg.args = global_args.clone();
        })?;
    }

    if !app_opts.is_empty() {
        config.app.update_config(|cfg| {
            cfg.opts = app_opts.clone();
        })?;
    }

    // Execute scripts
    execute_scripts(
        &selected_scripts,
        &global_args,
        &app_opts,
        &mut script_manager,
    )
}

fn collect_script_inputs(
    scripts: &[Script],
    global_args: &mut HashMap<String, serde_json::Value>,
    app_opts: &mut HashMap<String, serde_json::Value>,
) -> anyhow::Result<()> {
    for script in scripts {
        debug!("Collecting arguments for script: {}", script.name);
        // Collect script arguments
        if let Some(ref args) = script.args {
            for arg in args {
                if !global_args.contains_key(&arg.name) {
                    let value: String = Input::new()
                        .with_prompt(format!(
                            "Enter a directory path for {} - {}",
                            arg.name.cyan(),
                            arg.description
                        ))
                        .default(
                            dirs::home_dir()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                        )
                        .interact_text()?;

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
) -> anyhow::Result<()> {
    let colors = [
        colored::Color::Green,
        colored::Color::Yellow,
        colored::Color::Blue,
        colored::Color::Magenta,
        colored::Color::Cyan,
        colored::Color::Red,
    ];

    // RUST LEARNING: `enumerate()` gives (index, item) tuples (like Array.entries() in JS)
    for (index, script) in scripts.iter().enumerate() {
        // RUST LEARNING: Modulo operator for cycling through colors
        let color = colors[index % colors.len()];

        debug!("Executing script: {}", script.name);

        // RUST LEARNING: Method chaining - format!() creates String, .color() adds color
        println!("{}", format!("✨ Running {}...", script.name).color(color));

        // Prepare environment variables
        let mut env_vars = HashMap::new();

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

        debug!("Script env vars: {:?}", env_vars);

        // Prepare runtime and script
        let runtime_path = script_manager.prepare_runtime()?;
        let script_path = script_manager.prepare_script(script, "script")?;

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
        let mut cmd = Command::new(&runtime_path)
            .arg(&script_path)
            .stdin(stdio)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&env_vars) // Set all environment variables at once
            .spawn()?;

        // Handle output if not inheriting stdin
        if script.stdin.as_deref() != Some("inherit") {
            debug!("Spawning output handler threads");
            // RUST LEARNING: `take()` moves the value out of the Option, leaving None
            // - Like extracting a value from an object and nullifying it
            if let Some(stdout) = cmd.stdout.take() {
                let reader = BufReader::new(stdout);
                // RUST LEARNING: Clone data before moving into thread closure
                // - Threads require owned data, not references
                let script_name = script.pathname.clone();
                let color_clone = color;

                // RUST LEARNING: `thread::spawn()` creates a new OS thread
                // - `move` keyword transfers ownership into the closure
                // - Like Web Workers but for CPU-bound tasks
                thread::spawn(move || {
                    // RUST LEARNING: `map_while(Result::ok)` stops on first error
                    // - More efficient than collect() then iterate
                    for line in reader.lines().map_while(Result::ok) {
                        println!(
                            "{} {}",
                            format!("[{}]", script_name).color(color_clone),
                            line
                        );
                    }
                });
            }

            if let Some(stderr) = cmd.stderr.take() {
                let reader = BufReader::new(stderr);
                let script_name = script.pathname.clone();
                let color_clone = color;

                thread::spawn(move || {
                    for line in reader.lines().map_while(Result::ok) {
                        println!(
                            "{} {}",
                            format!("[{}]", script_name).color(color_clone),
                            line
                        );
                    }
                });
            }
        }

        let exit_status = cmd.wait()?;
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
