use buzz_kit::{cli::Cli, config};
use clap::Parser;

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let env: config::Env = std::env::vars()
        .filter(|(k, _)| k.starts_with("BUZZ_KIT_") || k == "CLAUDECODE" || k == "CODEX_THREAD_ID")
        .collect();
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is required"))?;
    let flags = config::Flags {
        assistant: cli.assistant,
        relay: cli.relay,
        config: cli.config,
    };
    let _config = config::load(
        std::path::Path::new(&home),
        &std::env::current_dir()?,
        &flags,
        &env,
    )?;
    anyhow::bail!("doctor is not implemented yet; M1 is in progress")
}

fn main() {
    if let Err(error) = run() {
        eprintln!("buzz-kit: {error}");
        std::process::exit(1);
    }
}
