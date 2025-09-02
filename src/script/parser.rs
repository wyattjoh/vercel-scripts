use crate::script::{
    types::{Script, ScriptArg, ScriptOpt, ScriptRequirement},
    Result, ScriptError,
};
use log::debug;
use regex::Regex;
use std::path::Path;

pub(crate) struct ScriptParser;

impl ScriptParser {
    /// Normalize a dependency path by removing leading "./" prefix
    pub fn normalize_dependency_path(dep: &str) -> String {
        if let Some(stripped) = dep.strip_prefix("./") {
            stripped.to_string()
        } else {
            dep.to_string()
        }
    }
    pub fn parse_script(content: &str, path: &Path, embedded: bool) -> Result<Script> {
        debug!("Parsing script: {}", path.display());

        let name = match Self::get_attribute(content, "name") {
            Some(name) => name,
            None => path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| ScriptError::InvalidPath(path.to_path_buf()))?,
        };

        let description = Self::get_attribute(content, "description");
        let after: Option<Vec<String>> = Self::get_attribute(content, "after")
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect());

        // Validate 'after' dependencies
        if let Some(ref deps) = after {
            for dep in deps {
                if dep.starts_with("../") {
                    return Err(ScriptError::InvalidDependencyPath(format!(
                        "Dependency '{}' uses parent directory reference which is not allowed",
                        dep
                    )));
                }
            }
        }

        let requires = Self::get_requires(content)?;
        let args = Self::get_args(content)?;
        let opts = Self::get_opts(content)?;
        let stdin = Self::get_stdin(content);

        debug!(
            "Script metadata - name: {}, args: {}, opts: {}, requires: {}",
            name,
            args.as_ref().map_or(0, |a| a.len()),
            opts.as_ref().map_or(0, |o| o.len()),
            requires.as_ref().map_or(0, |r| r.len())
        );

        if let Some(ref deps) = after {
            debug!("Script dependencies: {:?}", deps);
        }

        if let Some(ref reqs) = requires {
            debug!("Script requirements: {:?}", reqs);
        }

        Ok(Script {
            name,
            description,
            after,
            requires,
            absolute_pathname: path.to_path_buf(),
            pathname: path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .ok_or_else(|| ScriptError::InvalidPath(path.to_path_buf()))?,
            embedded,
            args,
            opts,
            stdin,
        })
    }

    fn get_attribute(content: &str, attribute: &str) -> Option<String> {
        let pattern = format!(r"@vercel\.{}\s+(.+)", attribute);
        let re = Regex::new(&pattern).ok()?;
        re.captures(content)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    fn get_args(content: &str) -> Result<Option<Vec<ScriptArg>>> {
        let re = Regex::new(r"(?m)@vercel\.arg\s+(?P<name>[A-Za-z0-9_]+)\s+(?P<description>.+)$")
            .expect("Invalid regex");

        let mut args = Vec::new();
        for caps in re.captures_iter(content) {
            let name = caps.name("name").unwrap().as_str().to_string();
            let description = caps
                .name("description")
                .unwrap()
                .as_str()
                .trim()
                .to_string();
            args.push(ScriptArg { name, description });
        }

        if args.is_empty() {
            Ok(None)
        } else {
            Ok(Some(args))
        }
    }

    fn get_opts(content: &str) -> Result<Option<Vec<ScriptOpt>>> {
        let re = Regex::new(r"(?m)@vercel\.opt\s+(?P<json>.+)$").expect("Invalid regex");

        let mut opts = Vec::new();
        for caps in re.captures_iter(content) {
            let json_str = caps.name("json").unwrap().as_str().trim();
            let opt: ScriptOpt = serde_json::from_str(json_str)
                .map_err(|e| ScriptError::InvalidScriptOption(format!("{}: {}", e, json_str)))?;
            opts.push(opt);
        }

        if opts.is_empty() {
            Ok(None)
        } else {
            Ok(Some(opts))
        }
    }

    fn get_stdin(content: &str) -> Option<String> {
        if content.contains("@vercel.stdin inherit") {
            Some("inherit".to_string())
        } else {
            None
        }
    }

    fn get_requires(content: &str) -> Result<Option<Vec<ScriptRequirement>>> {
        let re = Regex::new(r"(?m)@vercel\.requires\s+(?P<tokens>.+)$").expect("Invalid regex");

        let mut requirements = Vec::new();
        for caps in re.captures_iter(content) {
            if let Some(tokens_match) = caps.name("tokens") {
                let tokens: Vec<&str> = tokens_match.as_str().split_whitespace().collect();

                if tokens.is_empty() {
                    continue; // Skip empty requires lines
                }

                let script = tokens[0].to_string();

                // Validate that script dependency doesn't use parent directory reference
                if script.starts_with("../") {
                    return Err(ScriptError::InvalidDependencyPath(format!(
                        "Dependency '{}' uses parent directory reference which is not allowed",
                        script
                    )));
                }

                let variables = tokens[1..].iter().map(|&s| s.to_string()).collect();

                requirements.push(ScriptRequirement { script, variables });
            }
        }

        if requirements.is_empty() {
            Ok(None)
        } else {
            Ok(Some(requirements))
        }
    }
}
