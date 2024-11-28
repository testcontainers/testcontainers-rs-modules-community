use async_nats::connect;
use futures::StreamExt;
use testcontainers_modules::{nats::Nats, testcontainers::runners::AsyncRunner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    // startup the module
    let node = Nats::default().start().await?;

    // default docker username/password
    let default_username = "ruser";
    let default_password = "T0pS3cr3t";
    let topic = "foo";
    let port = node.get_host_port_ipv4(4222).await.unwrap();

    // prepare connection string
    let connection_url = &format!(
        "nats://{}:{}@127.0.0.1:{}",
        default_username, default_password, port,
    );

    let subscriber = connect(connection_url).await.unwrap();
    let publisher = connect(connection_url).await.unwrap();

    let mut subscription = subscriber.subscribe(topic).await.unwrap();

    println!("sending message");
    publisher.publish(topic, "Hello".into()).await.unwrap();
    publisher.publish(topic, "world".into()).await.unwrap();

    let mut messages = Vec::new();
    while let Some(message) = subscription.next().await {
        messages.push(message.clone());
        println!("Received message: {:?}", message);
        if messages.len() == 2 {
            break;
        }
    }

    assert_eq!(messages.len(), 2);
    println!("Received {} messages", messages.len());
    Ok(())
}
