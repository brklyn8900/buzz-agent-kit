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
    if matches!(cli.command, Command::Doctor) {
        let report = buzz_kit::doctor::run(home, &std::env::current_dir()?, &flags, &env);
        if cli.json {
            println!("{}", serde_json::to_string(&report)?);
        } else {
            for check in &report.checks {
                println!("{} {}: {}", check.status, check.name, check.message);
            }
        }
        if !report.ok {
            std::process::exit(1);
        }
        return Ok(());
    }
    let config = config::load(home, &std::env::current_dir()?, &flags, &env)?;
    match &cli.command {
        Command::Doctor => unreachable!(),
        Command::Assistant { command } => {
            let store = keystore::open(config.backend, home)?;
            match command {
                AssistantCommand::New { name, profile } => {
                    let scanner = buzz_kit::guards::Scanner::from_store(store.as_ref())?;
                    for value in [&profile.profile_name, &profile.about, &profile.avatar]
                        .into_iter()
                        .flatten()
                    {
                        scanner.check(value.as_bytes())?;
                    }
                    print_identity(&assistant::create(store.as_ref(), name)?, cli.json)?;
                    eprintln!(
                        "Send the public identity to your server operator. After membership, run buzz-kit assistant profile. No server request was made."
                    );
                    if profile.profile_name.is_some()
                        || profile.about.is_some()
                        || profile.avatar.is_some()
                    {
                        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
                        let mut command = format!(
                            "After membership: buzz-kit assistant profile {}",
                            quote(name)
                        );
                        for (flag, value) in [
                            ("--profile-name", &profile.profile_name),
                            ("--about", &profile.about),
                            ("--avatar", &profile.avatar),
                        ] {
                            if let Some(value) = value {
                                command.push_str(&format!(" {flag} {}", quote(value)));
                            }
                        }
                        eprintln!("{command}");
                    }
                }
                AssistantCommand::Profile { name, profile } => {
                    config::validate_name(name)?;
                    let secret = store
                        .get(name)?
                        .ok_or_else(|| anyhow::anyhow!("assistant key not found"))?;
                    let scanner = buzz_kit::guards::Scanner::from_store(store.as_ref())?;
                    let client = client(&config, &env, home)?;
                    println!("{}", client.profile(profile, &scanner, &secret)?);
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
        Command::Read { limit, thread } => {
            let channel =
                config.project.channel.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("project channel is not configured; run init")
                })?;
            let mut args = vec![
                "messages".into(),
                if thread.is_some() {
                    "thread".into()
                } else {
                    "get".into()
                },
                "--channel".into(),
                channel.id.clone(),
            ];
            if let Some(thread) = thread {
                args.extend(["--event".into(), thread.clone()]);
            }
            if let Some(limit) = limit {
                args.extend(["--limit".into(), limit.to_string()]);
            }
            read(&config, &env, home, None, &args)
        }
        Command::Search { query, author } => {
            let mut args = vec![
                "messages".into(),
                "search".into(),
                "--query".into(),
                query.clone(),
            ];
            if let Some(author) = author {
                args.extend(["--author".into(), author.clone()]);
            }
            read(&config, &env, home, None, &args)
        }
        Command::As { name, args } => read(&config, &env, home, Some(name), args),
    }
}
fn client(
    config: &config::Config,
    env: &config::Env,
    home: &Path,
) -> anyhow::Result<buzz_kit::buzz::Buzz> {
    Ok(buzz_kit::buzz::Buzz {
        executable: buzz_kit::buzz::discover(env, home)?,
        relay: config
            .relay
            .clone()
            .ok_or_else(|| anyhow::anyhow!("configure a relay URL"))?,
    })
}
fn read(
    config: &config::Config,
    env: &config::Env,
    home: &Path,
    name: Option<&str>,
    args: &[String],
) -> anyhow::Result<()> {
    buzz_kit::buzz::validate_read(args)?;
    let selected = name
        .map(String::from)
        .map(Ok)
        .unwrap_or_else(|| config::active_assistant(config, env))?;
    config::validate_name(&selected)?;
    let store = keystore::open(config.backend, home)?;
    let secret = store
        .get(&selected)?
        .ok_or_else(|| anyhow::anyhow!("assistant key not found"))?;
    println!("{}", client(config, env, home)?.read(args, &secret)?);
    Ok(())
}
fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(&cli) {
        if cli.json {
            eprintln!("{}", serde_json::json!({"error":error.to_string()}));
        } else {
            eprintln!("buzz-kit: {error}");
        }
        let exit = error
            .downcast_ref::<buzz_kit::buzz::Failure>()
            .and_then(|e| e.code)
            .filter(|c| *c > 0)
            .unwrap_or(1);
        std::process::exit(exit);
    }
}
