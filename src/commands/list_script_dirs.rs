use crate::config::Config;
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args)]
pub struct ListScriptDirsCommand;

impl ListScriptDirsCommand {
    pub fn execute(&self, config: &Config) -> anyhow::Result<()> {
        let current_config = config.global.get_config()?;

        if current_config.script_dirs.is_empty() {
            println!("{} No script directories configured", "Info:".blue());
            println!();
            println!(
                "Use {} to add a directory with scripts",
                "vss add-script-dir <directory>".cyan()
            );
            return Ok(());
        }

        println!("{} Script directories:", "Configured".green());
        println!();

        for (index, dir) in current_config.script_dirs.iter().enumerate() {
            let path = Path::new(dir);
            let exists = path.exists();
            let is_dir = path.is_dir();

            print!("  {}. {}", (index + 1).to_string().cyan(), dir);

            if !exists {
                print!(" {}", "(not found)".red());
            } else if !is_dir {
                print!(" {}", "(not a directory)".red());
            } else {
                // Count scripts in directory
                let script_count = self.count_scripts_in_directory(path)?;
                if script_count > 0 {
                    print!(
                        " {} {} script{}{}",
                        "→".green(),
                        script_count.to_string().cyan(),
                        if script_count == 1 { "" } else { "s" },
                        "".clear()
                    );
                } else {
                    print!(" {} {}", "→".yellow(), "no scripts".dimmed());
                }
            }

            println!();
        }

        println!();
        println!(
            "{} {} director{} configured",
            "Total:".dimmed(),
            current_config.script_dirs.len().to_string().cyan(),
            if current_config.script_dirs.len() == 1 {
                "y"
            } else {
                "ies"
            }
        );

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
