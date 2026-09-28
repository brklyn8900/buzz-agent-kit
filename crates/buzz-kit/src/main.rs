use buzz_kit::{
    assistant,
    cli::{AssistantCommand, Cli, Command},
    config,
    identity::Identity,
    keystore,
};
use clap::Parser;
use std::{
    io::{self, IsTerminal, Write},
    path::Path,
};

fn confirm(yes: bool, prompt: &str) -> anyhow::Result<bool> {
    if yes {
        return Ok(true);
    }
    if !io::stdin().is_terminal() {
        return Ok(false);
    }
    eprint!("{prompt} [y/N] ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}
fn print_identity(identity: &Identity, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string(identity)?);
    } else {
        println!("hex: {}\nnpub: {}", identity.hex, identity.npub);
    }
    Ok(())
}
fn run(cli: &Cli) -> anyhow::Result<()> {
    let env: config::Env = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .filter(|(k, _)| k.starts_with("BUZZ_KIT_") || k == "CLAUDECODE" || k == "CODEX_THREAD_ID")
        .collect();
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is required"))?;
    let home = Path::new(&home);
    let flags = config::Flags {
        assistant: cli.assistant.clone(),
        relay: cli.relay.clone(),
        config: cli.config.clone(),
    };
    let config = config::load(home, &std::env::current_dir()?, &flags, &env)?;
    match &cli.command {
        Command::Doctor => anyhow::bail!("doctor is not implemented yet; M1 is in progress"),
        Command::Assistant { command } => {
            let store = keystore::open(config.backend, home)?;
            match command {
                AssistantCommand::New { name } => {
                    print_identity(&assistant::create(store.as_ref(), name)?, cli.json)?;
                    eprintln!(
                        "Send the public identity to your server operator. After membership, run buzz-kit assistant profile. No server request was made."
                    );
                }
                AssistantCommand::Show { name } => {
                    print_identity(&assistant::show(store.as_ref(), name)?, cli.json)?
                }
                AssistantCommand::List => {
                    let mut identities = Vec::new();
                    for name in store.list()? {
                        let identity = assistant::show(store.as_ref(), &name)?;
                        if !cli.json {
                            println!("{name}  {}  {}", identity.hex, identity.npub);
                        }
                        identities.push(serde_json::json!({"name":name,"hex":identity.hex,"npub":identity.npub}));
                    }
                    if cli.json {
                        println!("{}", serde_json::to_string(&identities)?);
                    }
                }
                AssistantCommand::Remove { name, yes } => {
                    config::validate_name(name)?;
                    assistant::remove(
                        store.as_ref(),
                        name,
                        confirm(*yes, "Delete this assistant key? This cannot be undone.")?,
                    )?;
                    if cli.json {
                        println!("{}", serde_json::json!({"removed":name}));
                    } else {
                        println!("Removed {name}");
                    }
                }
                AssistantCommand::Import { legacy }
                | AssistantCommand::CleanupLegacy { legacy, .. } => {
                    anyhow::ensure!(
                        cfg!(target_os = "macos"),
                        "legacy Keychain migration is available only on macOS"
                    );
                    anyhow::ensure!(
                        matches!(
                            config.backend,
                            config::Backend::Auto | config::Backend::Keychain
                        ),
                        "legacy migration destination must be the macOS Keychain"
                    );
                    let (service, account) = assistant::legacy_parts(legacy)?;
                    let name = config.assistant.as_deref().unwrap_or(account);
                    let source = keystore::KeychainStore::new(
                        service,
                        home.join(".config/buzz-kit/identities/legacy-unused"),
                    )?;
                    match command {
                        AssistantCommand::Import { .. } => print_identity(
                            &assistant::import(&source, account, store.as_ref(), name)?,
                            cli.json,
                        )?,
                        AssistantCommand::CleanupLegacy { yes, .. } => {
                            assistant::cleanup(
                                &source,
                                account,
                                store.as_ref(),
                                name,
                                confirm(
                                    *yes,
                                    "Have all callers been re-verified? Delete the legacy copy?",
                                )?,
                            )?;
                            if cli.json {
                                println!("{}", serde_json::json!({"legacy_removed":true}));
                            } else {
                                println!("Legacy copy removed after matching destination key");
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Ok(())
        }
    }
}
fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(&cli) {
        if cli.json {
            eprintln!("{}", serde_json::json!({"error":error.to_string()}));
        } else {
            eprintln!("buzz-kit: {error}");
        }
        std::process::exit(1);
    }
}
