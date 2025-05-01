use testcontainers::{runners::AsyncRunner, ImageExt};
use testcontainers_modules::{postgres::Postgres, zitadel, zitadel::Zitadel};
extern crate pretty_env_logger;

const NETWORK: &str = "zitadel_network";
const POSTGRES_CONTAINER_NAME: &str = "postgres";
const ZITADEL_CONTAINER_NAME: &str = "zitadel";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    pretty_env_logger::init();

    let _postgres_node = Postgres::default()
        .with_host_auth()
        .with_db_name("postgres")
        .with_user("postgres")
        .with_password("postgres")
        .with_network(NETWORK)
        .with_container_name(POSTGRES_CONTAINER_NAME)
        .with_tag("15-alpine") // Use PostgreSQL 15
        .start()
        .await?;

    let zitadel_node = Zitadel::default()
        .with_postgres_database(
            Some(POSTGRES_CONTAINER_NAME.into()),
            Some(5432),
            Some("zitadel".into()),
        )
        .with_network(NETWORK)
        .with_container_name(ZITADEL_CONTAINER_NAME)
        .start()
        .await?;

    let host_port = zitadel_node
        .get_host_port_ipv4(zitadel::ZITADEL_PORT)
        .await?;
    println!("Zitadel is running on port: {}", host_port);

    Ok(())
}
