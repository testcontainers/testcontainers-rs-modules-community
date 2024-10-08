use testcontainers_modules::{mssql_server::MssqlServer, testcontainers::runners::AsyncRunner};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let image = MssqlServer::default();
    let container = image.start().await?;

    // Build Tiberius config
    let mut config = tiberius::Config::new();
    config.host(container.get_host().await?);
    config.port(container.get_host_port_ipv4(1433).await?);
    config.authentication(tiberius::AuthMethod::sql_server(
        "sa",
        MssqlServer::DEFAULT_SA_PASSWORD,
    ));
    config.trust_cert();

    // Connect to the database
    let tcp = TcpStream::connect(config.get_addr()).await?;
    tcp.set_nodelay(true)?;
    let mut client = tiberius::Client::connect(config, tcp.compat_write()).await?;

    // Run a test query
    let stream = client.query("SELECT 1 + 1", &[]).await?;
    let row = stream.into_row().await?.unwrap();
    assert_eq!(row.get::<i32, _>(0).unwrap(), 2);

    Ok(())
}
