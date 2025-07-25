use testcontainers::runners::AsyncRunner;
use testcontainers_modules::rqlite::RQLite;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let _ = pretty_env_logger::try_init();

    let node = RQLite::default().start().await?;
    let host_ip = node.get_host().await?;
    let host_port = node.get_host_port_ipv4(4001).await?;

    let client = rqlite_rs::RqliteClientBuilder::new()
        .known_host(format!("{}:{}", host_ip, host_port))
        .build()?;

    let query = rqlite_rs::query!("SELECT 1+1")?;
    let rows = client.fetch(query).await?;
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: i32 = first_row.get("1+1")?;
    assert_eq!(first_column, 2);
    Ok(())
}
