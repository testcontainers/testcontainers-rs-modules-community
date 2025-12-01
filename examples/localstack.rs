use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3 as s3;
use testcontainers_modules::{
    localstack::LocalStack,
    testcontainers::{runners::AsyncRunner, ImageExt},
};

#[tokio::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let _ = pretty_env_logger::try_init();

    let request = LocalStack::default().with_env_var("SERVICES", "s3");
    let container = request.start().await?;

    let host_ip = container.get_host().await?;
    let host_port = container.get_host_port_ipv4(4566).await?;
    // Set up AWS client
    let endpoint_url = format!("http://{host_ip}:{host_port}");
    let creds = s3::config::Credentials::new("fake", "fake", None, None, "test");

    let config = aws_sdk_s3::config::Builder::default()
        .behavior_version(BehaviorVersion::v2025_08_07())
        .region(Region::new("us-east-1"))
        .credentials_provider(creds)
        .endpoint_url(endpoint_url)
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);

    client
        .create_bucket()
        .bucket("example-bucket")
        .send()
        .await?;

    let list_buckets_output = client.list_buckets().send().await?;
    assert!(list_buckets_output.buckets.is_some());
    let buckets_list = list_buckets_output.buckets.unwrap();
    assert_eq!(1, buckets_list.len());
    assert_eq!("example-bucket", buckets_list[0].name.as_ref().unwrap());

    Ok(())
}
