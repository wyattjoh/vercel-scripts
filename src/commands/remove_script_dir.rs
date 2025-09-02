use crate::config::Config;
use clap::Args;
use colored::Colorize;
use inquire::{Confirm, Select};
use std::path::Path;

#[derive(Args)]
pub struct RemoveScriptDirCommand {
    /// Directory path to remove (optional, will prompt if not provided)
    path: Option<String>,

    /// Remove without confirmation
    #[arg(short, long)]
    yes: bool,
}

impl RemoveScriptDirCommand {
    pub fn execute(&self, config: &Config) -> anyhow::Result<()> {
        let current_config = config.global.get_config()?;

        if current_config.script_dirs.is_empty() {
            println!("{} No script directories configured", "Info:".blue());
            return Ok(());
        }

        let dir_to_remove = if let Some(ref path) = self.path {
            // Resolve the provided path
            let path = Path::new(path);
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                std::env::current_dir()?.join(path).canonicalize()?
            };
            let path_str = absolute_path.to_string_lossy().to_string();

            // Check if it exists in config
            if !current_config.script_dirs.contains(&path_str) {
                eprintln!(
                    "{} Directory not found in script directories: {}",
                    "Error:".red(),
                    path_str
                );
                eprintln!("Current script directories:");
                for dir in &current_config.script_dirs {
                    eprintln!("  - {}", dir);
                }
                std::process::exit(1);
            }

            path_str
        } else {
            // Interactive selection
            if current_config.script_dirs.len() == 1 {
                current_config.script_dirs[0].clone()
            } else {
                let selection = Select::new(
                    "Which script directory do you want to remove?",
                    current_config.script_dirs.clone(),
                )
                .prompt()?;

                selection
            }
        };

        // Confirm removal unless --yes flag is used
        if !self.yes {
            let confirm = Confirm::new(&format!("Remove script directory '{}'?", dir_to_remove))
                .with_default(false)
                .prompt()?;

            if !confirm {
                println!("Operation cancelled");
                return Ok(());
            }
        }

        // Remove from config
        config.global.update_config(|cfg| {
            cfg.script_dirs.retain(|dir| dir != &dir_to_remove);
        })?;

        println!(
            "{} Removed script directory: {}",
            "Success:".green(),
            dir_to_remove
        );

        let remaining_count = current_config.script_dirs.len() - 1;
        if remaining_count == 0 {
            println!("  No script directories remaining");
        } else {
            println!(
                "  {} script director{} remaining",
                remaining_count.to_string().cyan(),
                if remaining_count == 1 { "y" } else { "ies" }
            );
        }

        Ok(())
    }
}
