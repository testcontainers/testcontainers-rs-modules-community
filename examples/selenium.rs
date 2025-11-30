use testcontainers::runners::AsyncRunner;
use testcontainers_modules::selenium::Selenium;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let node = Selenium::new_firefox().start().await?;
    let driver_port = node
        .get_host_port_ipv4(testcontainers_modules::selenium::DRIVER_PORT)
        .await?;
    let driver_url = format!("http://127.0.0.1:{driver_port}");

    let client = fantoccini::ClientBuilder::native()
        .connect(&driver_url)
        .await?;

    let result = client.execute("return 2 + 2", vec![]).await?;
    let value = result.as_i64().unwrap();
    assert_eq!(value, 4);

    println!("Calculation result from Selenium: {}", value);

    client.close().await?;

    Ok(())
}
