use crate::config::Config;
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args)]
pub struct AddScriptDirCommand {
    /// Directory path to add
    path: String,
}

impl AddScriptDirCommand {
    pub fn execute(&self, config: &Config) -> anyhow::Result<()> {
        let path = Path::new(&self.path);

        // Validate that the directory exists
        if !path.exists() {
            eprintln!("{} Directory does not exist: {}", "Error:".red(), self.path);
            std::process::exit(1);
        }

        if !path.is_dir() {
            eprintln!("{} Path is not a directory: {}", "Error:".red(), self.path);
            std::process::exit(1);
        }

        // Convert to absolute path
        let absolute_path = path.canonicalize()?;
        let path_str = absolute_path.to_string_lossy().to_string();

        // Check if already added
        let current_config = config.global.get_config()?;
        if current_config.script_dirs.contains(&path_str) {
            println!(
                "{} Directory is already in script directories: {}",
                "Warning:".yellow(),
                path_str
            );
            return Ok(());
        }

        // Add to config
        config.global.update_config(|cfg| {
            cfg.script_dirs.push(path_str.clone());
        })?;

        println!(
            "{} Added script directory: {}",
            "Success:".green(),
            path_str
        );

        // Check for scripts in the directory
        let script_count = self.count_scripts_in_directory(path)?;
        if script_count > 0 {
            println!(
                "  Found {} script{} in directory",
                script_count.to_string().cyan(),
                if script_count == 1 { "" } else { "s" }
            );
        } else {
            println!("  {} No .sh scripts found in directory", "Note:".yellow());
        }

        Ok(())
    }

    fn count_scripts_in_directory(&self, dir: &Path) -> anyhow::Result<usize> {
        let mut count = 0;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sh") {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}
