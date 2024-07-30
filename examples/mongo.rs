use mongodb::{bson::doc, Client};
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::mongo::Mongo;

pub async fn get_connection_string(
    container: ContainerAsync<Mongo>,
) -> Result<String, testcontainers::core::error::TestcontainersError> {
    Ok(format!(
        "mongodb://{host}:{port}/",
        host = container.get_host().await?,
        port = container.get_host_port_ipv4(27017).await?,
    ))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Item {
    name: String,
    qty: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let _ = pretty_env_logger::try_init();
    // start a simple mongo server
    println!("creating a mongodb node...");
    let node = Mongo::default().start().await?;
    let host = node.get_host().await?;
    let port = node.get_host_port_ipv4(27017).await?;
    let client = Client::with_uri_str(format!("mongodb://{host}:{port}/")).await?;
    let col = client.database("test_db").collection::<Item>("test_col");

    let item = Item {
        name: "journal".to_string(),
        qty: 25,
    };
    println!("inserting item: {:?}", item);
    col.insert_one(item).await?;

    println!("finding item...");
    let found: Option<Item> = col
        .find_one(doc! {
            "name": "journal".to_string()
        })
        .await?;
    assert!(found.is_some());
    assert!(found.unwrap().qty == 25);
    println!("done!");

    println!("will try transactions...");
    println!("creating a mongodb with replica set node...");
    // start mongo server with replica set
    let node = Mongo::repl_set().start().await?;
    let host = node.get_host().await?;
    let port = node.get_host_port_ipv4(27017).await?;
    let client = Client::with_uri_str(format!(
        "mongodb://{host}:{port}/?directConnection=true&serverSelectionTimeoutMS=2000"
    ))
    .await?;

    let col = client.database("test_db").collection::<Item>("test_col");

    let item = Item {
        name: "mat".to_string(),
        qty: 85,
    };
    let mut session = client.start_session().await?;
    println!("starting transaction...");
    session.start_transaction().await?;
    println!("inserting item: {:?}", item);
    // we can use the transactions now
    col.insert_one(item).session(&mut session).await?;
    println!("committing...");
    session.commit_transaction().await?;

    println!("finding item...");
    let found: Option<Item> = col
        .find_one(doc! {
            "name": "mat".to_string()
        })
        .await?;
    assert!(found.is_some());
    assert!(found.unwrap().qty == 85);
    println!("done!");
    Ok(())
}
