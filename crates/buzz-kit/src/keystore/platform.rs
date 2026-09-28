use super::*;

pub struct KeychainStore {
    service: String,
    inventory: Inventory,
}
impl KeychainStore {
    pub fn new(service: &str, inventory: PathBuf) -> Result<Self> {
        validate_service(service)?;
        Ok(Self {
            service: service.to_owned(),
            inventory: Inventory {
                directory: inventory,
            },
        })
    }
    fn command(&self, verb: &str, name: &str) -> Result<Command> {
        validate_service(&self.service)?;
        validate_name(name)?;
        let mut command = Command::new("/usr/bin/security");
        command.args([verb, "-s", &self.service, "-a", name]);
        Ok(command)
    }
}
impl KeyStore for KeychainStore {
    fn put(&self, name: &str, secret: &Secret) -> Result<()> {
        validate_name(name)?;
        ensure!(
            self.get(name)?.is_none(),
            "assistant already exists; refusing replacement"
        );
        // Register before writing: interruption can leave a harmless stale name,
        // but never a key invisible to the exact-key scanner.
        self.inventory.register(name)?;
        let line = Zeroizing::new(format!(
            "add-generic-password -s {} -a {} -l \"buzz-kit {}\" -T /usr/bin/security -w {}\n",
            self.service,
            name,
            name,
            secret.expose()
        ));
        let result = capture(
            Command::new("/usr/bin/security").arg("-i"),
            Some(line.as_bytes()),
        )?;
        ensure!(
            result.code == Some(0),
            "Keychain creation failed; existing entries are never replaced"
        );
        match self.get(name) {
            Ok(Some(stored)) if stored.expose() == secret.expose() => Ok(()),
            _ => {
                self.delete(name).map_err(|_| anyhow::anyhow!("Keychain read-back failed and new-copy cleanup failed; legacy remains untouched"))?;
                anyhow::bail!("Keychain read-back verification failed; new copy removed")
            }
        }
    }
    fn get(&self, name: &str) -> Result<Option<Secret>> {
        let output = capture(self.command("find-generic-password", name)?.arg("-w"), None)?;
        match output.code {
            Some(0) => Ok(Some(Secret::from_bytes(&output.stdout)?)),
            Some(44) => Ok(None),
            _ => anyhow::bail!("Keychain read failed; unlock the login keychain and check access"),
        }
    }
    fn delete(&self, name: &str) -> Result<()> {
        let output = capture(&mut self.command("delete-generic-password", name)?, None)?;
        ensure!(
            matches!(output.code, Some(0 | 44)),
            "Keychain deletion failed"
        );
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for name in self.inventory.list()? {
            if self.get(&name)?.is_some() {
                names.push(name);
            }
        }
        Ok(names)
    }
}

pub struct SecretServiceStore {
    executable: PathBuf,
    inventory: Inventory,
}
impl SecretServiceStore {
    pub fn new(executable: PathBuf, inventory: PathBuf) -> Result<Self> {
        ensure!(
            executable.is_absolute(),
            "secret-tool path must be absolute"
        );
        Ok(Self {
            executable,
            inventory: Inventory {
                directory: inventory,
            },
        })
    }
    fn command(&self, verb: &str, name: &str) -> Result<Command> {
        validate_name(name)?;
        let mut command = Command::new(&self.executable);
        command.arg(verb);
        if verb == "store" {
            command.arg(format!("--label=buzz-kit {name}"));
        }
        command.args(["service", "buzz-kit", "account", name]);
        Ok(command)
    }
    fn lock(&self, name: &str) -> Result<Lock> {
        use std::os::unix::fs::OpenOptionsExt;
        validate_name(name)?;
        file::directory(&self.inventory.directory, true)?;
        let path = self.inventory.directory.join(format!(".lock-{name}"));
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).map_err(|_| anyhow::anyhow!("identity operation already in progress; a crashed helper may require removing its stale lock"))?;
        Ok(Lock(path))
    }
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
impl KeyStore for SecretServiceStore {
    fn put(&self, name: &str, secret: &Secret) -> Result<()> {
        let _lock = self.lock(name)?;
        ensure!(
            self.get(name)?.is_none(),
            "assistant already exists; refusing replacement"
        );
        self.inventory.register(name)?;
        let output = capture(
            &mut self.command("store", name)?,
            Some(secret.expose().as_bytes()),
        )?;
        if output.code == Some(0)
            && matches!(self.get(name), Ok(Some(s)) if s.expose() == secret.expose())
        {
            return Ok(());
        }
        // A failed store may have created an item; it was absent under our lock.
        let cleanup = capture(&mut self.command("clear", name)?, None)?;
        ensure!(
            cleanup.code == Some(0) || (cleanup.code == Some(1) && cleanup.stderr_empty),
            "secret-service creation failed and cleanup failed"
        );
        anyhow::bail!("secret-service creation or read-back failed; new copy removed")
    }
    fn get(&self, name: &str) -> Result<Option<Secret>> {
        let output = capture(&mut self.command("lookup", name)?, None)?;
        match output.code {
            Some(0) => Ok(Some(Secret::from_bytes(&output.stdout)?)),
            Some(1) if output.stdout.is_empty() && output.stderr_empty => Ok(None),
            _ => anyhow::bail!(
                "secret-service read failed; check the session service and unlock its collection"
            ),
        }
    }
    fn delete(&self, name: &str) -> Result<()> {
        let _lock = self.lock(name)?;
        let output = capture(&mut self.command("clear", name)?, None)?;
        ensure!(
            output.code == Some(0) || (output.code == Some(1) && output.stderr_empty),
            "secret-service deletion failed"
        );
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for name in self.inventory.list()? {
            if self.get(&name)?.is_some() {
                names.push(name);
            }
        }
        Ok(names)
    }
}
