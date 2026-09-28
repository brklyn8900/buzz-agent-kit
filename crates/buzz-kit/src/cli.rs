use clap::{Args, Parser, Subcommand};
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
    /// Install the verified Linux CLI or check the existing macOS app.
    InstallBuzzCli,
    /// Explicitly update, roll back, or release a persistent binary pin.
    Update {
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        unpin: bool,
        #[arg(long)]
        no_prune: bool,
    },
    /// Configure this project and merge pinned teammate plugin settings.
    Init {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        no_verify: bool,
        #[arg(long)]
        no_claude_settings: bool,
        #[arg(long)]
        update_claude_settings: bool,
    },
    /// Scan and post the exact file/stdin bytes to the project channel.
    Post {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        thread: Option<String>,
        #[arg(long)]
        split: bool,
        #[arg(long)]
        kind: Option<u16>,
        /// A UTF-8 message file, or - for stdin. Attachments are not supported.
        file: String,
    },
    /// Read channel messages or a thread.
    Read {
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        thread: Option<String>,
    },
    Search {
        query: String,
        #[arg(long)]
        author: Option<String>,
    },
    /// Run an allowlisted read command with this assistant's identity.
    As {
        name: String,
        #[arg(last = true, required = true)]
        args: Vec<String>,
    },
    /// Manage assistant identities; private keys are never printed.
    Assistant {
        #[command(subcommand)]
        command: AssistantCommand,
    },
}

#[derive(Subcommand)]
pub enum AssistantCommand {
    New {
        name: String,
        #[command(flatten)]
        profile: Profile,
    },
    Profile {
        name: String,
        #[command(flatten)]
        profile: Profile,
    },
    List,
    Show {
        name: String,
    },
    Remove {
        name: String,
        #[arg(long)]
        yes: bool,
    },
    /// Copy a legacy Keychain item; never deletes its source.
    Import {
        #[arg(long)]
        legacy: String,
    },
    /// Delete the old copy only after all callers have been re-verified.
    CleanupLegacy {
        legacy: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Args, Clone, Default)]
#[group(id = "profile-fields")]
pub struct Profile {
    #[arg(long)]
    pub profile_name: Option<String>,
    #[arg(long)]
    pub about: Option<String>,
    #[arg(long)]
    pub avatar: Option<String>,
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
