use crate::error::VssResult;
use crate::script::ScriptOpt;
use crate::worktree::WorktreeManager;
use colored::Colorize;
use inquire::{Confirm, Select, Text};
use std::collections::HashMap;
use std::path::Path;

/// Handle a boolean script option by prompting the user
pub(crate) fn handle_boolean_option(opt: &ScriptOpt, default: &Option<bool>) -> VssResult<bool> {
    let value = Confirm::new(opt.description())
        .with_default(default.unwrap_or(false))
        .prompt()?;
    Ok(value)
}

/// Handle a string script option with optional pattern validation
pub(crate) fn handle_string_option(
    opt: &ScriptOpt,
    default: &Option<String>,
    pattern: &Option<String>,
    pattern_help: &Option<String>,
) -> VssResult<Option<String>> {
    let value = loop {
        let mut input = Text::new(opt.description());

        if let Some(def) = default {
            input = input.with_default(def);
        }

        let input_value: String = input.prompt()?;

        // Handle validation like TypeScript version
        if let Some(pattern) = pattern {
            let re = regex::Regex::new(pattern).map_err(anyhow::Error::from)?;

            // If empty and optional, skip validation
            if input_value.is_empty() && opt.is_optional() {
                break input_value;
            }

            if re.is_match(&input_value) {
                break input_value;
            } else {
                // Show pattern help if available, otherwise default message
                let error_msg = if let Some(help) = pattern_help {
                    help.clone()
                } else {
                    "Invalid input format".to_string()
                };
                println!("{}", error_msg.red());
                continue;
            }
        }

        // Check for empty values on required fields
        if input_value.is_empty() && !opt.is_optional() {
            let error_msg = if pattern_help.is_some() {
                pattern_help.as_deref().unwrap()
            } else {
                "Value is required"
            };
            println!("{}", error_msg.red());
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
    default: &Option<String>,
    global_args: &HashMap<String, serde_json::Value>,
) -> VssResult<Option<String>> {
    if let Some(base_dir_value) = global_args.get(base_dir_arg) {
        if let serde_json::Value::String(base_dir) = base_dir_value {
            let worktrees = WorktreeManager::list_worktrees(base_dir).unwrap_or_default();

            // Only prompt if there are worktrees or if not optional (like TypeScript version)
            if !worktrees.is_empty() || !opt.is_optional() {
                let choices: Vec<String> = worktrees
                    .iter()
                    .map(|wt| wt.display_name(Path::new(base_dir)))
                    .collect();

                // Find default index based on default value
                let default_idx = if let Some(default_val) = default {
                    worktrees
                        .iter()
                        .position(|wt| wt.path.to_string_lossy() == *default_val)
                        .unwrap_or(0)
                } else {
                    0
                };

                let selection = Select::new(opt.description(), choices)
                    .with_starting_cursor(default_idx)
                    .prompt()?;

                // Find the worktree that matches the selection
                if let Some(worktree) = worktrees
                    .iter()
                    .find(|wt| wt.display_name(Path::new(base_dir)) == selection)
                {
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

    // Return default value if available, otherwise None
    Ok(default.clone())
}
