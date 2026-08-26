use base64::{prelude::BASE64_STANDARD, Engine};
use rand::rngs::ThreadRng;
use rand::Rng;
use sha2::{Digest as Sha512Digest, Sha512};
use std::{borrow::Cow, collections::HashMap};
use testcontainers::core::error::Result;
use testcontainers::core::{CmdWaitFor, ContainerPort, ContainerState, ExecCommand};
use testcontainers::{core::WaitFor, CopyToContainer, Image};

const NAME: &str = "registry.suse.com/suse/389-ds";
const TAG: &str = "3.0";

const LDAP_PORT: ContainerPort = ContainerPort::Tcp(3389);
const LDAPS_PORT: ContainerPort = ContainerPort::Tcp(3636);

/// Module to work with [`389-ds`] inside of tests.
///
/// Starts an instance of 389-ds.
/// This module is based on the official [`389-ds docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{port389, testcontainers::runners::SyncRunner};
///
/// let port389ds_instance = Port389DS::default().with_user("bind_test", "changeme").start().unwrap();
///
/// let connection_string = format!(
///     "ldap://{}:{}",
///     port389ds_instance.get_host().unwrap(),
///     port389ds_instance.get_host_port_ipv4(3389).unwrap(),
/// );
/// ```
///
/// [`389-ds`]: https://www.port389.org/
/// [`389-ds docker image`]: registry.suse.com/suse/389-ds
#[derive(Debug, Clone)]
pub struct Port389DS {
    env_vars: HashMap<String, String>,
    copy_to_sources: Vec<CopyToContainer>,
    suffix: String,
    users: Vec<User>,
    config: Vec<String>,
}

#[derive(Debug, Clone)]
struct User {
    username: String,
    password: String,
}

impl Port389DS {
    /// Sets the password for the 389ds directory manager.
    pub fn with_dm_password(mut self, password: &str) -> Self {
        self.env_vars
            .insert("DS_DM_PASSWORD".to_owned(), password.to_owned());
        self
    }

    /// Adds a user (can be called multiple times)
    /// Default: `[]`
    ///
    pub fn with_user(mut self, username: impl ToString, password: impl ToString) -> Self {
        self.users.push(User {
            username: username.to_string(),
            password: password.to_string(),
        });
        self
    }

    /// Adds a config option (can be alled multiple times)
    /// Default: `[]`
    ///
    pub fn with_config(mut self, option: impl ToString) -> Self {
        self.config.push(option.to_string());
        self
    }
}

fn generate_salt() -> [u8; 4] {
    ThreadRng::default().next_u32().to_ne_bytes()
}

/// Generates a secure {SSHA512} (Salted SHA-512) hash for LDAP
fn generate_ssha512(password: &str) -> String {
    let salt = generate_salt();

    // Hash password + salt
    let mut hasher = Sha512::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    let hash_result = hasher.finalize();

    // Combine hash bytes and salt bytes
    let mut combined = Vec::with_capacity(hash_result.len() + salt.len());
    combined.extend_from_slice(&hash_result);
    combined.extend_from_slice(&salt);

    // Encode to base64 and add LDAP prefix
    let b64 = BASE64_STANDARD.encode(combined);
    format!("{{SSHA512}}{}", b64)
}

impl Default for Port389DS {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("DS_DM_PASSWORD".to_owned(), "changeme".to_owned());

        Self {
            env_vars,
            copy_to_sources: Vec::new(),
            suffix: "dc=example,dc=org".to_owned(),
            users: Vec::new(),
            config: Vec::new(),
        }
    }
}

impl Image for Port389DS {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("INFO: 389-ds-container started")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn copy_to_sources(&self) -> impl IntoIterator<Item = &CopyToContainer> {
        &self.copy_to_sources
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[LDAPS_PORT, LDAP_PORT]
    }

    fn exec_after_start(&self, _: ContainerState) -> Result<Vec<ExecCommand>> {
        let create_backend_cmd = ExecCommand::new(vec![
            "dsconf",
            "localhost",
            "backend",
            "create",
            "--suffix",
            &self.suffix,
            "--be-name",
            "userroot",
            "--create-suffix",
            "--create-entries",
        ])
        .with_cmd_ready_condition(CmdWaitFor::exit_code(0));

        let mut commands = vec![create_backend_cmd];

        let mut uid = 1000;
        for user in &self.users {
            uid += 1;
            let create_user = ExecCommand::new(vec![
                "dsidm",
                "localhost",
                "--basedn",
                &self.suffix,
                "user",
                "create",
                "--uid",
                &user.username,
                "--cn",
                &user.username,
                "--displayName",
                &user.username,
                "--uidNumber",
                &uid.to_string(),
                "--gidNumber",
                &uid.to_string(),
                "--homeDirectory",
                format!("/home/{}", user.username).as_str(),
            ])
            .with_cmd_ready_condition(CmdWaitFor::exit_code(0));
            commands.push(create_user);

            let password_hash = generate_ssha512(&user.password);
            let set_pwd = ExecCommand::new(vec![
                "dsidm",
                "localhost",
                "--basedn",
                &self.suffix,
                "user",
                "modify",
                &user.username,
                format!("add:userPassword:{}", password_hash.trim()).as_str(),
            ])
            .with_cmd_ready_condition(CmdWaitFor::exit_code(0));

            commands.push(set_pwd);
        }

        for entry in &self.config {
            let cmd = ExecCommand::new(vec!["dsconf", "localhost", "config", "replace", entry])
                .with_cmd_ready_condition(CmdWaitFor::exit_code(0));
            commands.push(cmd);
        }

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ldap3::{LdapConnAsync, LdapConnSettings};
    use reqwest::Url;
    use std::time::Duration;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::ImageExt;

    #[tokio::test]
    async fn port389ds_connect() -> std::result::Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let port389ds_image = Port389DS::default()
            .with_user("bind_test", "changeme")
            .with_startup_timeout(Duration::from_secs(180));

        let node = port389ds_image.start().await?;

        let connection_string = format!(
            "ldap://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(LDAP_PORT).await?,
        );

        let settings = LdapConnSettings::new().set_conn_timeout(Duration::from_secs(1));
        let (conn, mut ldap) =
            LdapConnAsync::from_url_with_settings(settings, &Url::parse(&connection_string)?)
                .await?;

        ldap3::drive!(conn);
        ldap.with_timeout(Duration::from_secs(1))
            .simple_bind("uid=bind_test,ou=people,dc=example,dc=org", "changeme")
            .await?
            .success()?;

        ldap.unbind().await?;

        Ok(())
    }
}
