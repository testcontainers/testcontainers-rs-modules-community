use serde::{Deserialize, Serialize};
use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};
use testcontainers_modules::{
    surrealdb::{SurrealDb, SURREALDB_PORT},
    testcontainers::runners::AsyncRunner,
};

#[derive(Debug, Serialize, Deserialize)]
struct Name {
    first: String,
    last: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Person {
    title: String,
    name: Name,
    marketing: bool,
}

#[tokio::main]
async fn main() {
    let _ = pretty_env_logger::try_init();
    let node = SurrealDb::default().start().await.unwrap();
    let url = format!(
        "127.0.0.1:{}",
        node.get_host_port_ipv4(SURREALDB_PORT).await.unwrap()
    );

    let db: Surreal<Client> = Surreal::init();
    db.connect::<Ws>(url)
        .await
        .expect("Failed to connect to SurrealDB");
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await
    .expect("Failed to signin to SurrealDB");

    db.use_ns("test")
        .use_db("test")
        .await
        .expect("Failed to use test db");

    db.create::<Option<Person>>(("person", "tobie"))
        .content(Person {
            title: "Founder & CEO".to_string(),
            name: Name {
                first: "Tobie".to_string(),
                last: "Morgan Hitchcock".to_string(),
            },
            marketing: true,
        })
        .await
        .expect("Failed to create Tobie :(");

    let result = db
        .select::<Option<Person>>(("person", "tobie"))
        .await
        .expect("Failed to select Tobie :(");

    assert!(result.is_some());
    let result = result.expect("Failed to unwrap Tobie :(");

    assert_eq!(result.title, "Founder & CEO");
    assert_eq!(result.name.first, "Tobie");
    assert_eq!(result.name.last, "Morgan Hitchcock");
    assert!(result.marketing);

    println!("All right, all right, all right!\n\n{:#?}", result);
}
