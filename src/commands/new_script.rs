use crate::config::Config;
use crate::error::{VssError, VssResult};
use crate::script::{
    types::{Script, ScriptArg, ScriptOpt, ScriptOptType, ScriptRequirement},
    ScriptManager,
};
use clap::Args;
use colored::Colorize;
use inquire::{validator::Validation, Confirm, MultiSelect, Select, Text};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct NewScriptCommand;

struct ScriptMetadata<'a> {
    shell_type: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    dependencies: &'a [String],
    requirements: &'a [ScriptRequirement],
    args: &'a [ScriptArg],
    opts: &'a [ScriptOpt],
    stdin_mode: Option<&'a str>,
}

impl NewScriptCommand {
    pub fn execute(&self, config: &Config) -> VssResult<()> {
        let config_data = config.global.get_config().map_err(anyhow::Error::from)?;

        if config_data.script_dirs.is_empty() {
            eprintln!(
                "{} No script directories configured. Add one with 'vss add-script-dir <path>'",
                "Error:".red()
            );
            std::process::exit(1);
        }

        println!("{}", "Creating a new Vercel script...".cyan().bold());
        println!();

        // 1. Select target directory
        let target_dir = self.select_target_directory(&config_data.script_dirs)?;

        // 2. Get script filename
        let filename = self.get_script_filename(&target_dir)?;
        let script_path = target_dir.join(&filename);

        // 3. Get script metadata
        let script_name = self.get_script_name(&filename)?;
        let description = self.get_script_description()?;
        let shell_type = self.select_shell_type()?;

        // 4. Load existing scripts for dependency selection
        let mut script_manager = ScriptManager::new();
        let existing_scripts = script_manager
            .get_scripts(&config_data.script_dirs)
            .map_err(anyhow::Error::from)?;

        // 5. Configure dependencies
        let dependencies = self.select_dependencies(&existing_scripts)?;

        // 6. Configure requirements
        let requirements = self.configure_requirements(&existing_scripts)?;

        // 7. Configure arguments
        let args = self.configure_arguments()?;

        // 8. Configure options
        let opts = self.configure_options(&args)?;

        // 9. Configure stdin
        let stdin_mode = self.configure_stdin()?;

        // 10. Generate and write script
        let metadata = ScriptMetadata {
            shell_type: &shell_type,
            name: &script_name,
            description: description.as_deref(),
            dependencies: &dependencies,
            requirements: &requirements,
            args: &args,
            opts: &opts,
            stdin_mode: stdin_mode.as_deref(),
        };
        let script_content = self.generate_script_content(&metadata);

        fs::write(&script_path, script_content)
            .map_err(|e| anyhow::anyhow!("Failed to write script: {}", e))?;

        // Make the script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)
                .map_err(|e| anyhow::anyhow!("Failed to get file permissions: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)
                .map_err(|e| anyhow::anyhow!("Failed to set file permissions: {}", e))?;
        }

        println!();
        println!(
            "{} Created script: {}",
            "Success:".green(),
            script_path.display()
        );
        println!("  Name: {}", script_name.cyan());
        if let Some(desc) = description {
            println!("  Description: {}", desc);
        }
        if !dependencies.is_empty() {
            println!("  Dependencies: {}", dependencies.join(", ").bright_black());
        }
        if !args.is_empty() {
            println!("  Arguments: {}", args.len().to_string().bright_black());
        }
        if !opts.is_empty() {
            println!("  Options: {}", opts.len().to_string().bright_black());
        }

        Ok(())
    }

    fn select_target_directory(&self, script_dirs: &[String]) -> VssResult<PathBuf> {
        if script_dirs.len() == 1 {
            return Ok(PathBuf::from(&script_dirs[0]));
        }

        let selection =
            Select::new("Select target script directory", script_dirs.to_vec()).prompt()?;

        Ok(PathBuf::from(&selection))
    }

    fn get_script_filename(&self, target_dir: &Path) -> VssResult<String> {
        loop {
            let filename = Text::new("Script filename (without .sh extension):").prompt()?;

            if filename.is_empty() {
                eprintln!("{} Filename cannot be empty", "Error:".red());
                continue;
            }
            if filename.contains('/') || filename.contains('\\') {
                eprintln!("{} Filename cannot contain path separators", "Error:".red());
                continue;
            }
            if filename.ends_with(".sh") {
                eprintln!("{} Don't include .sh extension", "Error:".red());
                continue;
            }

            let full_filename = format!("{}.sh", filename);
            let script_path = target_dir.join(&full_filename);

            if script_path.exists() {
                eprintln!(
                    "{} File already exists: {}",
                    "Error:".red(),
                    script_path.display()
                );
                continue;
            }

            return Ok(full_filename);
        }
    }

    fn get_script_name(&self, filename: &str) -> VssResult<String> {
        let default_name = filename
            .strip_suffix(".sh")
            .unwrap_or(filename)
            .replace(['_', '-'], " ");

        let script_name = Text::new("Script name:")
            .with_default(&default_name)
            .prompt()?;

        if script_name.trim().is_empty() {
            return Err(VssError::Other(anyhow::anyhow!(
                "Script name cannot be empty"
            )));
        }

        Ok(script_name)
    }

    fn get_script_description(&self) -> VssResult<Option<String>> {
        let description = Text::new("Description (optional):")
            .with_default("")
            .prompt()?;

        Ok(if description.trim().is_empty() {
            None
        } else {
            Some(description)
        })
    }

    fn select_shell_type(&self) -> VssResult<String> {
        let shells = vec!["zsh", "bash"];
        let selection = Select::new("Shell type:", shells).prompt()?;

        Ok(selection.to_string())
    }

    fn select_dependencies(&self, existing_scripts: &[Script]) -> VssResult<Vec<String>> {
        if existing_scripts.is_empty() {
            return Ok(Vec::new());
        }

        let add_dependencies = Confirm::new("Add script dependencies (@vercel.after)?")
            .with_default(false)
            .prompt()?;

        if !add_dependencies {
            return Ok(Vec::new());
        }

        let script_names: Vec<String> = existing_scripts
            .iter()
            .map(|s| format!("./{}", s.pathname))
            .collect();

        let selections = MultiSelect::new(
            "Select dependencies (scripts that must run before this one):",
            script_names.clone(),
        )
        .prompt()?;

        Ok(selections)
    }

    fn configure_requirements(
        &self,
        existing_scripts: &[Script],
    ) -> VssResult<Vec<ScriptRequirement>> {
        let mut requirements = Vec::new();

        if existing_scripts.is_empty() {
            return Ok(requirements);
        }

        let add_requirements = Confirm::new("Add script requirements (@vercel.requires)?")
            .with_default(false)
            .prompt()?;

        if !add_requirements {
            return Ok(requirements);
        }

        loop {
            let script_names: Vec<String> = existing_scripts
                .iter()
                .map(|s| format!("./{}", s.pathname))
                .collect();

            let script_name = Select::new("Select required script:", script_names).prompt()?;

            let variables_input = Text::new("Required variables (space-separated):").prompt()?;

            if variables_input.trim().is_empty() {
                eprintln!("{} At least one variable is required", "Error:".red());
                continue;
            }

            let variables: Vec<String> = variables_input
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            requirements.push(ScriptRequirement {
                script: script_name,
                variables,
            });

            let add_another = Confirm::new("Add another requirement?")
                .with_default(false)
                .prompt()?;

            if !add_another {
                break;
            }
        }

        Ok(requirements)
    }

    fn configure_arguments(&self) -> VssResult<Vec<ScriptArg>> {
        let mut args = Vec::new();

        let add_args = Confirm::new("Add script arguments (@vercel.arg)?")
            .with_default(false)
            .prompt()?;

        if !add_args {
            return Ok(args);
        }

        loop {
            let name = loop {
                let input = Text::new("Argument name (environment variable):").prompt()?;

                if input.trim().is_empty() {
                    eprintln!("{} Argument name cannot be empty", "Error:".red());
                    continue;
                }
                if !input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    eprintln!("{} Argument name must contain only alphanumeric characters and underscores", "Error:".red());
                    continue;
                }
                break input;
            };

            let description = loop {
                let input = Text::new("Argument description:").prompt()?;

                if input.trim().is_empty() {
                    eprintln!("{} Argument description cannot be empty", "Error:".red());
                    continue;
                }
                break input;
            };

            args.push(ScriptArg { name, description });

            let add_another = Confirm::new("Add another argument?")
                .with_default(false)
                .prompt()?;

            if !add_another {
                break;
            }
        }

        Ok(args)
    }

    fn configure_options(&self, args: &[ScriptArg]) -> VssResult<Vec<ScriptOpt>> {
        let mut opts = Vec::new();

        let add_opts = Confirm::new("Add script options (@vercel.opt)?")
            .with_default(false)
            .prompt()?;

        if !add_opts {
            return Ok(opts);
        }

        loop {
            let option_types = ScriptOptType::all();
            let option_type = Select::new("Option type:", option_types).prompt()?;

            let name = loop {
                let input = Text::new("Option name (environment variable):").prompt()?;

                if input.trim().is_empty() {
                    eprintln!("{} Option name cannot be empty", "Error:".red());
                    continue;
                }
                if !input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    eprintln!(
                        "{} Option name must contain only alphanumeric characters and underscores",
                        "Error:".red()
                    );
                    continue;
                }
                break input;
            };

            let description = loop {
                let input = Text::new("Option description:").prompt()?;

                if input.trim().is_empty() {
                    eprintln!("{} Option description cannot be empty", "Error:".red());
                    continue;
                }
                break input;
            };

            let optional = Confirm::new("Is this option optional?")
                .with_default(true)
                .prompt()?;

            match option_type {
                ScriptOptType::Boolean => {
                    let default = if Confirm::new("Set a default value?")
                        .with_default(false)
                        .prompt()?
                    {
                        let default_value = Confirm::new("Default value:")
                            .with_default(false)
                            .prompt()?;
                        Some(default_value)
                    } else {
                        None
                    };

                    opts.push(ScriptOpt::Boolean {
                        name,
                        description,
                        default,
                        optional,
                    });
                }
                ScriptOptType::String => {
                    let default = if Confirm::new("Set a default value?")
                        .with_default(false)
                        .prompt()?
                    {
                        let default_value =
                            Text::new("Default value:").with_default("").prompt()?;
                        if default_value.is_empty() {
                            None
                        } else {
                            Some(default_value)
                        }
                    } else {
                        None
                    };

                    let (pattern, pattern_help) = if Confirm::new("Add validation pattern (regex)?")
                        .with_default(false)
                        .prompt()?
                    {
                        let pattern_str = Text::new("Validation pattern (regex):")
                            .with_validator(|input: &str| match Regex::new(input) {
                                Ok(_) => Ok(Validation::Valid),
                                Err(e) => Ok(Validation::Invalid(
                                    format!("Invalid regex pattern: {}", e).into(),
                                )),
                            })
                            .prompt()?;

                        let pattern_help_str = Text::new("Pattern help text (optional):")
                            .with_default("")
                            .prompt()?;

                        let help = if pattern_help_str.is_empty() {
                            None
                        } else {
                            Some(pattern_help_str)
                        };

                        (Some(pattern_str), help)
                    } else {
                        (None, None)
                    };

                    opts.push(ScriptOpt::String {
                        name,
                        description,
                        default,
                        optional,
                        pattern,
                        pattern_help,
                    });
                }
                ScriptOptType::Worktree => {
                    let base_dir_arg = Select::new(
                        "Select base directory argument:",
                        args.iter().map(|arg| arg.name.clone()).collect(),
                    )
                    .prompt()?;

                    opts.push(ScriptOpt::Worktree {
                        name,
                        description,
                        base_dir_arg,
                        optional,
                    });
                }
            }

            let add_another = Confirm::new("Add another option?")
                .with_default(false)
                .prompt()?;

            if !add_another {
                break;
            }
        }

        Ok(opts)
    }

    fn configure_stdin(&self) -> VssResult<Option<String>> {
        let add_stdin = Confirm::new("Configure stdin handling (@vercel.stdin)?")
            .with_default(false)
            .prompt()?;

        if !add_stdin {
            return Ok(None);
        }

        let inherit_stdin = Confirm::new("Inherit stdin?").with_default(true).prompt()?;

        Ok(if inherit_stdin {
            Some("inherit".to_string())
        } else {
            None
        })
    }

    fn generate_script_content(&self, metadata: &ScriptMetadata) -> String {
        let mut content = String::new();

        // Shebang
        content.push_str(&format!("#!/usr/bin/env {}\n\n", metadata.shell_type));

        // Script annotations
        content.push_str(&format!("# @vercel.name {}\n", metadata.name));

        if let Some(desc) = metadata.description {
            content.push_str(&format!("# @vercel.description {}\n", desc));
        }

        if !metadata.dependencies.is_empty() {
            content.push_str(&format!(
                "# @vercel.after {}\n",
                metadata.dependencies.join(" ")
            ));
        }

        for req in metadata.requirements {
            content.push_str(&format!(
                "# @vercel.requires {} {}\n",
                req.script,
                req.variables.join(" ")
            ));
        }

        for arg in metadata.args {
            content.push_str(&format!("# @vercel.arg {} {}\n", arg.name, arg.description));
        }

        for opt in metadata.opts {
            let opt_json = serde_json::to_string(&opt).unwrap();
            content.push_str(&format!("# @vercel.opt {}\n", opt_json));
        }

        if let Some(stdin) = metadata.stdin_mode {
            content.push_str(&format!("# @vercel.stdin {}\n", stdin));
        }

        content.push('\n');

        // Script body
        content.push_str("set -e\n\n");
        content.push_str("# TODO: Implement your script logic here\n");

        content
    }
}
