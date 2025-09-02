// RUST LEARNING: `use` statements are like TypeScript imports but work differently
// - They bring items into scope from crates (packages) and modules
// - `clap` is like a TypeScript CLI library (similar to commander.js)
// - `vss` refers to our own crate (defined in lib.rs)
use clap::{Parser, Subcommand};
use std::env;
use vss::{
    run_scripts, AddScriptDirCommand, CompletionsCommand, Config, ListScriptDirsCommand,
    ListScriptsCommand, NewScriptCommand, RemoveScriptDirCommand, VssError, VERSION,
};

// RUST LEARNING: `#[derive]` is a macro that auto-generates code
// - Like TypeScript decorators but more powerful and run at compile-time
// - `Parser` generates command-line parsing code for the struct
// - These `#[command]` attributes configure the CLI behavior
#[derive(Parser)]
#[command(name = "vss")]
#[command(
    about = "Vercel Scripts Selector - Interactive script runner for Vercel development workflows"
)]
#[command(version = VERSION)]
struct Cli {
    // RUST LEARNING: `Option<T>` is like TypeScript's `T | undefined`
    // - Rust forces explicit handling of nullable values (no null/undefined crashes!)
    #[command(subcommand)]
    command: Option<Commands>,

    /// Replay the last run without prompts
    #[arg(short, long)]
    replay: bool,

    /// Enable debug logging for script operations
    #[arg(short = 'd', long, global = true)]
    debug: bool,
}

// RUST LEARNING: `enum` in Rust is like TypeScript unions but much more powerful
// - Each variant can hold different types of data (like tagged unions)
// - This is more like `type Commands = { type: 'add', data: AddCommand } | { type: 'remove', data: RemoveCommand }`
#[derive(Subcommand)]
enum Commands {
    /// Add a script directory
    #[command(name = "add-script-dir")]
    AddScriptDir(AddScriptDirCommand), // Holds an AddScriptDirCommand struct

    /// Remove a script directory
    #[command(name = "remove-script-dir")]
    RemoveScriptDir(RemoveScriptDirCommand),

    /// List configured script directories
    #[command(name = "list-script-dirs")]
    ListScriptDirs(ListScriptDirsCommand),

    /// List all available scripts
    #[command(name = "list-scripts", alias = "ls")]
    ListScripts(ListScriptsCommand),

    /// Create a new script with guided prompts
    New(NewScriptCommand),

    /// Generate shell completions
    Completions(CompletionsCommand),
}

// RUST LEARNING: Function returns `Result<(), Error>` instead of throwing exceptions
// - `anyhow::Result<()>` is like `Promise<void>` that can fail
// - `()` is Rust's unit type (like `void` in TypeScript)
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on debug flag
    if cli.debug {
        env::set_var("RUST_LOG", "vss=debug");
    }
    env_logger::init();

    // RUST LEARNING: The `?` operator is like `await` for Results
    // - If Config::new() fails, it immediately returns the error
    // - No try/catch needed - handled by the type system
    let config = Config::new()?;

    match cli.command {
        Some(Commands::AddScriptDir(cmd)) => cmd.execute(&config),
        Some(Commands::RemoveScriptDir(cmd)) => match cmd.execute(&config) {
            Ok(()) => Ok(()),
            Err(VssError::UserInterrupted) => {
                std::process::exit(0);
            }
            Err(VssError::Other(err)) => Err(err),
        },
        Some(Commands::ListScriptDirs(cmd)) => cmd.execute(&config),
        Some(Commands::ListScripts(cmd)) => cmd.execute(&config),
        Some(Commands::New(cmd)) => match cmd.execute(&config) {
            Ok(()) => Ok(()),
            Err(VssError::UserInterrupted) => {
                std::process::exit(0);
            }
            Err(VssError::Other(err)) => Err(err),
        },
        Some(Commands::Completions(cmd)) => {
            cmd.generate_completions::<Cli>();
            Ok(())
        }
        // RUST LEARNING: `None` handles the case where command is undefined/null
        None => match run_scripts(cli.replay, cli.debug, &config) {
            Ok(()) => Ok(()),
            Err(VssError::UserInterrupted) => {
                std::process::exit(0);
            }
            Err(VssError::Other(err)) => Err(err),
        },
    }
}
