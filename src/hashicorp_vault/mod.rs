use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{core::WaitFor, Image};

const DEFAULT_IMAGE_NAME: &str = "hashicorp/vault";
const DEFAULT_IMAGE_TAG: &str = "1.17";

/// Module to work with [`Hashicorp Vault`] inside of tests.
///
/// This module is based on the official [`Hashicorp Vault docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{hashicorp_vault, testcontainers::runners::SyncRunner};
///
/// let vault = hashicorp_vault::HashicorpVault::default().start().unwrap();
/// let http_port = vault.get_host_port_ipv4(8200).unwrap();
///
/// // do something with the running vault instance..
/// ```
///
/// [`Hashicorp Vault`]: https://github.com/hashicorp/vault
/// [`Hashicorp Vault docker image`]: https://hub.docker.com/r/hashicorp/vault
/// [`Hashicorp Vault commands`]: https://developer.hashicorp.com/vault/docs/commands
#[derive(Debug, Clone)]
pub struct HashicorpVault {
    name: String,
    tag: String,
    env_vars: BTreeMap<String, String>,
}

impl Default for HashicorpVault {
    /**
     * Starts an in-memory instance in dev mode, with horrible token values.
     * Obviously not to be emulated in production.
     */
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();
        env_vars.insert("VAULT_DEV_ROOT_TOKEN_ID".to_string(), "myroot".to_string());
        HashicorpVault::new(
            DEFAULT_IMAGE_NAME.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
            env_vars,
        )
    }
}

impl HashicorpVault {
    fn new(name: String, tag: String, env_vars: BTreeMap<String, String>) -> Self {
        HashicorpVault {
            name,
            tag,
            env_vars,
        }
    }
}

impl Image for HashicorpVault {
    fn name(&self) -> &str {
        &self.name
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Vault server started!")]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use vaultrs::{
        client::{VaultClient, VaultClientSettingsBuilder},
        kv2,
    };

    use super::*;
    use crate::testcontainers::runners::AsyncRunner;

    // Create and read secrets
    #[derive(Debug, Deserialize, Serialize)]
    struct MySecret {
        key: String,
        password: String,
    }

    #[tokio::test]
    async fn hashicorp_vault_secret_set_and_read(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let vault = HashicorpVault::default().start().await.unwrap();
        let endpoint = format!("http://0.0.0.0:{}", vault.get_host_port_ipv4(8200).await?);

        // Create a client
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(endpoint)
                .token("myroot")
                .build()
                .unwrap(),
        )
        .unwrap();

        let secret = MySecret {
            key: "super".to_string(),
            password: "secret".to_string(),
        };
        kv2::set(&client, "secret", "mysecret", &secret).await?;

        let secret: MySecret = kv2::read(&client, "secret", "mysecret").await.unwrap();
        assert_eq!(secret.key, "super");
        assert_eq!(secret.password, "secret");
        Ok(())
    }
}
