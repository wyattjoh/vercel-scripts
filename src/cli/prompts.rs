use crate::script::ScriptOpt;
use crate::worktree::WorktreeManager;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use std::collections::HashMap;
use std::path::Path;

/// Handle a boolean script option by prompting the user
pub(crate) fn handle_boolean_option(
    opt: &ScriptOpt,
    default: &Option<bool>,
) -> anyhow::Result<bool> {
    let value = Confirm::new()
        .with_prompt(opt.description())
        .default(default.unwrap_or(false))
        .interact()?;
    Ok(value)
}

/// Handle a string script option with optional pattern validation
pub(crate) fn handle_string_option(
    opt: &ScriptOpt,
    default: &Option<String>,
    pattern: &Option<String>,
    pattern_help: &Option<String>,
) -> anyhow::Result<Option<String>> {
    let value = loop {
        let mut input = Input::new().with_prompt(opt.description());

        if let Some(def) = default {
            input = input.default(def.clone());
        }

        let input_value: String = input.interact_text()?;

        if let Some(pattern) = pattern {
            let re = regex::Regex::new(pattern)?;
            if re.is_match(&input_value) || (input_value.is_empty() && opt.is_optional()) {
                break input_value;
            } else {
                println!(
                    "{}",
                    pattern_help
                        .as_deref()
                        .unwrap_or("Invalid input format")
                        .red()
                );
                continue;
            }
        }

        if input_value.is_empty() && !opt.is_optional() {
            println!("{}", "Value is required".red());
            continue;
        }

        break input_value;
    };

    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

/// Handle a worktree script option by listing available worktrees
pub(crate) fn handle_worktree_option(
    opt: &ScriptOpt,
    base_dir_arg: &str,
    global_args: &HashMap<String, serde_json::Value>,
) -> anyhow::Result<Option<String>> {
    if let Some(base_dir_value) = global_args.get(base_dir_arg) {
        if let serde_json::Value::String(base_dir) = base_dir_value {
            let worktrees = WorktreeManager::list_worktrees(base_dir).unwrap_or_default();

            if !worktrees.is_empty() || !opt.is_optional() {
                let mut choices = vec!["(Use base directory)".to_string()];
                choices.extend(
                    worktrees
                        .iter()
                        .map(|wt| wt.display_name(Path::new(base_dir))),
                );

                let selection = Select::new()
                    .with_prompt(opt.description())
                    .items(&choices)
                    .default(0)
                    .interact()?;

                if selection > 0 {
                    let worktree = &worktrees[selection - 1];
                    return Ok(Some(worktree.path.to_string_lossy().to_string()));
                }
            }
        }
    } else {
        println!(
            "{} Base directory {} not set, skipping {}",
            "Warning:".yellow(),
            base_dir_arg,
            opt.name()
        );
    }

    Ok(None)
}
