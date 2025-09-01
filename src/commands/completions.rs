use crate::config::Config;
use clap::Args;
use clap_complete::{generate, Shell};
use std::io;

#[derive(Args)]
pub struct CompletionsCommand {
    /// The shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

impl CompletionsCommand {
    pub fn execute(&self, _config: &Config) -> anyhow::Result<()> {
        // This method exists for consistency with other commands
        // The actual completion generation is handled by generate_completions()
        Ok(())
    }

    /// Generate completions for the given command
    pub fn generate_completions<C: clap::CommandFactory>(&self) {
        let mut cmd = C::command();
        let name = cmd.get_name().to_string();
        generate(self.shell, &mut cmd, name, &mut io::stdout());
    }
}
