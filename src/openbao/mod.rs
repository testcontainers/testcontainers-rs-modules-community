use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const DEFAULT_IMAGE_NAME: &str = "quay.io/openbao/openbao";
const DEFAULT_IMAGE_TAG: &str = "2.5.5";

/// Module to work with [`OpenBao`] inside of tests.
///
/// This module is based on the official [`OpenBao container image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{openbao, testcontainers::runners::SyncRunner};
///
/// let openbao = openbao::OpenBao::default().start().unwrap();
/// let http_port = openbao.get_host_port_ipv4(8200).unwrap();
///
/// // do something with the running OpenBao instance..
/// ```
///
/// [`OpenBao`]: https://github.com/openbao/openbao
/// [`OpenBao container image`]: https://quay.io/openbao/openbao
/// [`OpenBao commands`]: https://openbao.org/docs/commands/
#[derive(Debug, Clone)]
pub struct OpenBao {
    name: String,
    tag: String,
    env_vars: BTreeMap<String, String>,
}

impl Default for OpenBao {
    /**
     * Starts an in-memory instance in dev mode, with root token set to "root".
     * Obviously not to be emulated in production.
     */
    fn default() -> Self {
        let mut env_vars = BTreeMap::new();
        env_vars.insert("BAO_DEV_ROOT_TOKEN_ID".to_string(), "root".to_string());
        OpenBao::new(
            DEFAULT_IMAGE_NAME.to_string(),
            DEFAULT_IMAGE_TAG.to_string(),
            env_vars,
        )
    }
}

impl OpenBao {
    fn new(name: String, tag: String, env_vars: BTreeMap<String, String>) -> Self {
        OpenBao {
            name,
            tag,
            env_vars,
        }
    }
}

impl Image for OpenBao {
    fn name(&self) -> &str {
        &self.name
    }

    fn tag(&self) -> &str {
        &self.tag
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        let http_strategy = HttpWaitStrategy::new("v1/sys/health")
            .with_port(ContainerPort::Tcp(8200))
            .with_response_matcher(|resp| resp.status().as_u16() == 200);
        vec![WaitFor::http(http_strategy)]
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
    async fn openbao_secret_set_and_read() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let openbao = OpenBao::default().start().await.unwrap();
        let endpoint = format!("http://0.0.0.0:{}", openbao.get_host_port_ipv4(8200).await?);

        // Create a client
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(endpoint)
                .token("root")
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
