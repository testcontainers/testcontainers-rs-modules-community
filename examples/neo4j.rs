use testcontainers_modules::{neo4j::Neo4j, testcontainers::runners::AsyncRunner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let container = Neo4j::default().start().await;

    // prepare neo4rs client
    let config = neo4rs::ConfigBuilder::new()
        .uri(format!(
            "bolt://{}:{}",
            container.get_host_ip_address().await,
            container.image().bolt_port_ipv4()
        ))
        .user(container.image().user().expect("default user is set"))
        .password(
            container
                .image()
                .password()
                .expect("default password is set"),
        )
        .build()?;

    // connect ot Neo4j
    let graph = neo4rs::Graph::connect(config).await?;

    // run a test query
    let mut rows = graph.execute(neo4rs::query("RETURN 1 + 1")).await?;
    while let Some(row) = rows.next().await? {
        let result: i64 = row.get("1 + 1").unwrap();
        assert_eq!(result, 2);
    }

    Ok(())
}
