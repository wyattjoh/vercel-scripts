use crate::error::VssResult;
use crate::script::ScriptOpt;
use crate::worktree::WorktreeManager;
use crate::VssError;
use colored::Colorize;
use inquire::validator::Validation;
use inquire::{Confirm, Select, Text};
use std::collections::HashMap;

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
    let optional = opt.is_optional();
    let pattern_owned = pattern.clone();
    let pattern_help_owned = pattern_help.clone();

    let mut input = Text::new(opt.description()).with_validator(move |input: &str| {
        if let Some(pattern) = &pattern_owned {
            let re = regex::Regex::new(pattern.as_str()).map_err(anyhow::Error::from)?;
            if re.is_match(input) {
                Ok(Validation::Valid)
            } else if let Some(pattern_help) = &pattern_help_owned {
                Ok(Validation::Invalid(pattern_help.into()))
            } else {
                Ok(Validation::Invalid(
                    format!(
                        "Input did not match the expected pattern: {}",
                        pattern.as_str()
                    )
                    .into(),
                ))
            }
        } else if input.is_empty() && optional {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid("Value is required".into()))
        }
    });

    if let Some(def) = default {
        input = input.with_default(def);
    }

    let value = input.prompt()?;

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
    existing_args: &HashMap<String, serde_json::Value>,
) -> VssResult<Option<String>> {
    if let Some(base_dir_value) = existing_args.get(base_dir_arg) {
        if let serde_json::Value::String(base_dir) = base_dir_value {
            let worktrees = WorktreeManager::list_worktrees(base_dir).unwrap_or_default();

            if !worktrees.is_empty() {
                let selection = Select::new(opt.description(), worktrees).prompt()?;

                return Ok(Some(selection.path.to_string_lossy().to_string()));
            } else if !opt.is_optional() {
                return Err(VssError::Other(anyhow::anyhow!(
                    "No worktrees found for base directory {}",
                    base_dir
                )));
            }

            return Ok(None);
        }

        return Err(VssError::Other(anyhow::anyhow!(
            "Base directory argument {} is required, but not set",
            base_dir_arg
        )));
    } else if opt.is_optional() {
        println!(
            "{} Base directory argument {} not set, skipping {}",
            "Warning:".yellow(),
            base_dir_arg,
            opt.name()
        );

        return Ok(None);
    } else {
        return Err(VssError::Other(anyhow::anyhow!(
            "Base directory argument {} is required, but not set",
            base_dir_arg
        )));
    }
}
