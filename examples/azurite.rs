use azure_storage::{prelude::*, CloudLocation};
use azure_storage_blobs::prelude::*;
use futures::stream::StreamExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::azurite::{Azurite, BLOB_PORT};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let container = Azurite::default().start().await?;
    let container_client = ClientBuilder::with_location(
        CloudLocation::Emulator {
            address: "127.0.0.1".to_owned(),
            port: container.get_host_port_ipv4(BLOB_PORT).await?,
        },
        StorageCredentials::emulator(),
    )
    .container_client("container-name");

    container_client.create().await?;
    let blob_client = container_client.blob_client("blob-name");
    blob_client
        .put_block_blob("hello world")
        .content_type("text/plain")
        .await?;
    let mut result: Vec<u8> = vec![];

    // The stream is composed of individual calls to the get blob endpoint
    let mut stream = blob_client.get().into_stream();
    while let Some(value) = stream.next().await {
        let mut body = value?.data;
        // For each response, we stream the body instead of collecting it all
        // into one large allocation.
        while let Some(value) = body.next().await {
            let value = value?;
            result.extend(&value);
        }
    }

    println!("result: {:?}", result);

    Ok(())
}
