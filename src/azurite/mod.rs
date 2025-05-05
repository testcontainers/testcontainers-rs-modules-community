use std::{borrow::Cow, collections::BTreeMap};

use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "mcr.microsoft.com/azure-storage/azurite";
const TAG: &str = "3.34.0";

/// Port that [`Azurite`] uses internally for blob storage.
pub const BLOB_PORT: ContainerPort = ContainerPort::Tcp(10000);

/// Port that [`Azurite`] uses internally for queue.
pub const QUEUE_PORT: ContainerPort = ContainerPort::Tcp(10001);

/// Port that [`Azurite`] uses internally for table.
const TABLE_PORT: ContainerPort = ContainerPort::Tcp(10002);

const AZURITE_ACCOUNTS: &str = "AZURITE_ACCOUNTS";

/// Module to work with [`Azurite`] inside tests.
///
/// This module is based on the official [`Azurite docker image`].
///
/// # Example
/// ```
/// use testcontainers_modules::{
///     azurite,
///     azurite::{Azurite, BLOB_PORT},
///     testcontainers::runners::SyncRunner,
/// };
///
/// let azurite = Azurite::default().start().unwrap();
/// let blob_port = azurite.get_host_port_ipv4(BLOB_PORT).unwrap();
///
/// // do something with the started azurite instance..
/// ```
///
/// [`Azurite`]: https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azurite?toc=%2Fazure%2Fstorage%2Fblobs%2Ftoc.json&bc=%2Fazure%2Fstorage%2Fblobs%2Fbreadcrumb%2Ftoc.json&tabs=visual-studio%2Cblob-storage
/// [`Azurite docker image`]: https://hub.docker.com/r/microsoft/azure-storage-azurite
#[derive(Debug, Clone)]
pub struct Azurite {
    env_vars: BTreeMap<String, String>,
    loose: bool,
    skip_api_version_check: bool,
    disable_telemetry: bool,
}

impl Default for Azurite {
    fn default() -> Self {
        Self {
            env_vars: BTreeMap::new(),
            loose: false,
            skip_api_version_check: false,
            disable_telemetry: false,
        }
    }
}
impl Azurite {
    /// Sets the [Azurite accounts](https://learn.microsoft.com/en-us/azure/storage/common/storage-use-azurite?toc=%2Fazure%2Fstorage%2Fblobs%2Ftoc.json&bc=%2Fazure%2Fstorage%2Fblobs%2Fbreadcrumb%2Ftoc.json&tabs=visual-studio%2Ctable-storage#custom-storage-accounts-and-keys) to be used by the instance.
    ///
    /// - Uses `AZURITE_ACCOUNTS` key is used to store the accounts in the environment variables.
    /// - The format should be: `account1:key1[:key2];account2:key1[:key2];...`
    pub fn with_accounts(self, accounts: String) -> Self {
        let mut env_vars = self.env_vars;
        env_vars.insert(AZURITE_ACCOUNTS.to_owned(), accounts);
        Self { env_vars, ..self }
    }

    /// Disables strict mode
    pub fn with_loose(self) -> Self {
        Self {
            loose: true,
            ..self
        }
    }

    /// Skips API version validation
    pub fn with_skip_api_version_check(self) -> Self {
        Self {
            skip_api_version_check: true,
            ..self
        }
    }

    /// Disables telemetry data collection
    pub fn with_disable_telemetry(self) -> Self {
        Self {
            disable_telemetry: true,
            ..self
        }
    }
}
impl Image for Azurite {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout(
            "Azurite Table service is successfully listening at http://0.0.0.0:10002",
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<Cow<'_, str>>> {
        let mut cmd = vec![
            String::from("azurite"),
            String::from("--blobHost"),
            String::from("0.0.0.0"),
            String::from("--queueHost"),
            String::from("0.0.0.0"),
            String::from("--tableHost"),
            String::from("0.0.0.0"),
        ];
        if self.loose {
            cmd.push(String::from("--loose"));
        }
        if self.skip_api_version_check {
            cmd.push(String::from("--skipApiVersionCheck"));
        }
        if self.disable_telemetry {
            cmd.push(String::from("--disableTelemetry"));
        }
        cmd
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[BLOB_PORT, QUEUE_PORT, TABLE_PORT]
    }
}

#[cfg(test)]
mod tests {
    use azure_storage::{prelude::*, CloudLocation};
    use azure_storage_blobs::prelude::*;
    use base64::{prelude::BASE64_STANDARD, Engine};

    use crate::azurite::{Azurite, BLOB_PORT};

    #[tokio::test]
    async fn starts_with_async_runner() -> Result<(), Box<dyn std::error::Error + 'static>> {
        use testcontainers::runners::AsyncRunner;
        let azurite = Azurite::default();
        azurite.start().await?;
        Ok(())
    }

    #[test]
    fn starts_with_sync_runner() -> Result<(), Box<dyn std::error::Error + 'static>> {
        use testcontainers::runners::SyncRunner;
        let azurite = Azurite::default();
        azurite.start()?;
        Ok(())
    }

    #[test]
    fn starts_with_loose() -> Result<(), Box<dyn std::error::Error + 'static>> {
        use testcontainers::runners::SyncRunner;
        let azurite = Azurite::default().with_loose();
        azurite.start()?;
        Ok(())
    }

    #[test]
    fn starts_with_with_skip_api_version_check() -> Result<(), Box<dyn std::error::Error + 'static>>
    {
        use testcontainers::runners::SyncRunner;
        let azurite = Azurite::default().with_skip_api_version_check();
        azurite.start()?;
        Ok(())
    }

    #[tokio::test]
    async fn starts_with_accounts() -> Result<(), Box<dyn std::error::Error + 'static>> {
        use azure_core::auth::Secret;
        use testcontainers::runners::AsyncRunner;

        let data = b"key1";
        let account_key = BASE64_STANDARD.encode(data);

        let account_name = "account1";
        let container = Azurite::default()
            .with_accounts(format!("{}:{};", account_name, account_key))
            .start()
            .await?;

        ClientBuilder::with_location(
            CloudLocation::Custom {
                account: account_name.to_string(),
                uri: format!(
                    "http://127.0.0.1:{}/{}",
                    container.get_host_port_ipv4(BLOB_PORT).await?,
                    account_name
                ),
            },
            StorageCredentials::access_key(account_name, Secret::new(account_key)),
        )
        .container_client("container-name")
        .create()
        .await?;

        Ok(())
    }
}
