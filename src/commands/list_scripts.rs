use crate::config::Config;
use crate::script::ScriptManager;
use clap::Args;
use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, ContentArrangement, Table};

#[derive(Args)]
pub struct ListScriptsCommand;

impl ListScriptsCommand {
    pub fn execute(&self, config: &Config) -> anyhow::Result<()> {
        let current_config = config.global.get_config()?;
        let mut script_manager = ScriptManager::new();

        let scripts = script_manager.get_scripts(&current_config.script_dirs)?;

        if scripts.is_empty() {
            println!("{} No scripts found.", "Info:".yellow());
            println!();
            println!(
                "  Use {} to add a directory with scripts",
                "vss add-script-dir <directory>".cyan()
            );
            return Ok(());
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL);
        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Set headers
        table.set_header(vec![
            Cell::new("Name").fg(comfy_table::Color::Green),
            Cell::new("Description").fg(comfy_table::Color::Green),
            Cell::new("Source").fg(comfy_table::Color::Green),
            Cell::new("Arguments").fg(comfy_table::Color::Green),
            Cell::new("Options").fg(comfy_table::Color::Green),
        ]);

        for script in &scripts {
            let source = if script.embedded {
                Cell::new("embedded").fg(comfy_table::Color::Blue)
            } else {
                let dir = script
                    .absolute_pathname
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                Cell::new(dir).fg(comfy_table::Color::Cyan)
            };

            let args = if let Some(ref args) = script.args {
                let arg_names: Vec<String> = args.iter().map(|arg| arg.name.clone()).collect();
                arg_names.join(", ")
            } else {
                "none".dimmed().to_string()
            };

            let opts = if let Some(ref opts) = script.opts {
                let opt_names: Vec<String> =
                    opts.iter().map(|opt| opt.name().to_string()).collect();
                opt_names.join(", ")
            } else {
                "none".dimmed().to_string()
            };

            let description = script
                .description
                .as_deref()
                .unwrap_or("No description")
                .to_string();

            table.add_row(vec![
                Cell::new(&script.name),
                Cell::new(description),
                source,
                Cell::new(args),
                Cell::new(opts),
            ]);
        }

        println!("{}", table);
        println!();
        println!(
            "{} {} script{} found",
            "Total:".dimmed(),
            scripts.len().to_string().cyan(),
            if scripts.len() == 1 { "" } else { "s" }
        );

        // Show breakdown by source
        let embedded_count = scripts.iter().filter(|s| s.embedded).count();
        let external_count = scripts.len() - embedded_count;

        if embedded_count > 0 && external_count > 0 {
            println!(
                "  {} embedded, {} external",
                embedded_count.to_string().blue(),
                external_count.to_string().cyan()
            );
        } else if embedded_count > 0 {
            println!(
                "  {} embedded script{}",
                embedded_count.to_string().blue(),
                if embedded_count == 1 { "" } else { "s" }
            );
        } else {
            println!(
                "  {} external script{}",
                external_count.to_string().cyan(),
                if external_count == 1 { "" } else { "s" }
            );
        }

        Ok(())
    }
}
