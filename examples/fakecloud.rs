use testcontainers_modules::{
    fakecloud::{FakeCloud, FAKECLOUD_PORT},
    testcontainers::runners::AsyncRunner,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let _ = pretty_env_logger::try_init();

    let node = FakeCloud::default().start().await?;
    let host_ip = node.get_host().await?;
    let host_port = node.get_host_port_ipv4(FAKECLOUD_PORT).await?;

    println!("fakecloud running at http://{}:{}", host_ip, host_port);

    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "http://{}:{}/_fakecloud/health",
            host_ip, host_port
        ))
        .send()
        .await?;

    println!("Health check: {}", response.text().await?);

    Ok(())
}
