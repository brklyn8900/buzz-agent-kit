use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Guarded collaboration through Buzz")]
pub struct Cli {
    #[arg(long = "as", global = true)]
    pub assistant: Option<String>,
    #[arg(long, global = true)]
    pub relay: Option<String>,
    #[arg(long, global = true)]
    pub json: bool,
    /// Project config path (defaults to the nearest .buzz/config.json).
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check local setup and server access.
    Doctor,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_flags_work_after_subcommand() {
        let cli = Cli::try_parse_from([
            "buzz-kit",
            "doctor",
            "--as",
            "r2d2",
            "--json",
            "--config",
            "project.json",
        ])
        .unwrap();
        assert_eq!(cli.assistant.as_deref(), Some("r2d2"));
        assert!(cli.json);
        assert_eq!(cli.config, Some(PathBuf::from("project.json")));
    }
}
