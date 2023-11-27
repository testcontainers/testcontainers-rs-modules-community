use std::collections::HashMap;

use testcontainers::{core::WaitFor, Image};

/// [Microsoft SQL Server](https://www.microsoft.com/en-us/sql-server) module
/// for [testcontainers](https://crates.io/crates/testcontainers).
///
/// This module is based on the
/// [official Microsoft SQL Server for Linux Docker image](https://hub.docker.com/_/microsoft-mssql-server).
/// Only amd64 images are available for SQL Server. If you use Apple silicon machines,
/// you need to configure Rosetta emulation.
///
/// * [Change Docker Desktop settings on Mac | Docker Docs](https://docs.docker.com/desktop/settings/mac/#general)
///
/// # Example
///
/// ```
/// use testcontainers::clients;
/// use testcontainers_modules::mssql_server;
///
/// let docker = clients::Cli::default();
/// let mssql_server = docker.run(mssql_server::MssqlServer::default());
/// let ado_connection_string = format!(
///    "Server=tcp:127.0.0.1,{};Database=test;User Id=sa;Password=yourStrong(!)Password;TrustServerCertificate=True;",
///    mssql_server.get_host_port_ipv4(1433)
/// );
/// ```
///
/// # Environment variables
///
/// Refer to the [documentation](https://learn.microsoft.com/en-us/sql/linux/sql-server-linux-configure-environment-variables)
/// for a complite list of environment variables.
///
/// Following environment variables are required.
/// A image provided by this module has default values for them.
///
/// ## `ACCEPT_EULA`
///
/// You need to accept the [End-User Licensing Agreement](https://go.microsoft.com/fwlink/?linkid=857698)
/// before using the SQL Server image provided by this module.
/// To accept EULA, you can set this environment variable to `Y`.
/// The default value is `Y`.
///
/// ## `SA_PASSWORD`
///
/// The SA user password. This password is required to conform to the
/// [strong password policy](https://learn.microsoft.com/en-us/sql/relational-databases/security/password-policy?view=sql-server-ver15#password-complexity).
/// The default value is `yourStrong(!)Password`.
///
/// ## `MSSQL_PID`
///
/// The edition of SQL Server.
/// The default value is `Developer`, which will run the container using the Developer Edition.
#[derive(Debug)]
pub struct MssqlServer {
    env_vars: HashMap<String, String>,
}

impl MssqlServer {
    const NAME: &'static str = "mcr.microsoft.com/mssql/server";
    const TAG: &'static str = "2022-CU10-ubuntu-22.04";
    const DEFAULT_SA_PASSWORD: &'static str = "yourStrong(!)Password";
}

impl Default for MssqlServer {
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("ACCEPT_EULA".to_owned(), "Y".to_owned());
        env_vars.insert(
            "SA_PASSWORD".to_owned(),
            Self::DEFAULT_SA_PASSWORD.to_owned(),
        );
        env_vars.insert("MSSQL_PID".to_owned(), "Developer".to_owned());

        Self { env_vars }
    }
}

impl Image for MssqlServer {
    type Args = ();

    fn name(&self) -> String {
        Self::NAME.to_owned()
    }

    fn tag(&self) -> String {
        Self::TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::StdOutMessage {
            message: "SQL Server is now ready for client connections".to_owned(),
        }]
    }

    fn env_vars(&self) -> Box<dyn Iterator<Item = (&String, &String)> + '_> {
        Box::new(self.env_vars.iter())
    }
}

#[cfg(test)]
mod tests {
    use std::error;

    use testcontainers::{clients, RunnableImage};
    use tiberius::{AuthMethod, Client, Config};
    use tokio::net::TcpStream;
    use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

    use super::*;

    #[tokio::test]
    async fn one_plus_one() -> Result<(), Box<dyn error::Error>> {
        let docker = clients::Cli::default();
        let container = docker.run(MssqlServer::default());
        let config = new_config(container.get_host_port_ipv4(1433));
        let mut client = get_mssql_client(config).await?;

        let stream = client.query("SELECT 1 + 1", &[]).await?;
        let row = stream.into_row().await?.unwrap();

        assert_eq!(row.get::<i32, _>(0).unwrap(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn custom_version() -> Result<(), Box<dyn error::Error>> {
        let docker = clients::Cli::default();
        let image = RunnableImage::from(MssqlServer::default()).with_tag("2019-CU23-ubuntu-20.04");
        let container = docker.run(image);
        let config = new_config(container.get_host_port_ipv4(1433));
        let mut client = get_mssql_client(config).await?;

        let stream = client.query("SELECT @@VERSION", &[]).await?;
        let row = stream.into_row().await?.unwrap();

        assert!(row.get::<&str, _>(0).unwrap().contains("2019"));

        Ok(())
    }

    async fn get_mssql_client(
        config: Config,
    ) -> Result<Client<Compat<TcpStream>>, Box<dyn error::Error>> {
        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write()).await?;

        Ok(client)
    }

    fn new_config(port: u16) -> Config {
        let mut config = Config::new();
        config.port(port);
        config.authentication(AuthMethod::sql_server("sa", "yourStrong(!)Password"));
        config.trust_cert();

        config
    }
}
