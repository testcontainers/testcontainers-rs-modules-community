use testcontainers::{core::WaitFor, Image};

const NAME: &str = "amazon/dynamodb-local";
const TAG: &str = "2.0.0";
const DEFAULT_WAIT: u64 = 3000;

#[derive(Default, Debug)]
pub struct DynamoDb;

impl Image for DynamoDb {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stdout(
                "Initializing DynamoDB Local with the following configuration",
            ),
            WaitFor::millis(DEFAULT_WAIT),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_dynamodb::{
        config::Credentials,
        types::{
            AttributeDefinition, KeySchemaElement, KeyType, ProvisionedThroughput,
            ScalarAttributeType,
        },
        Client,
    };

    use crate::{dynamodb_local::DynamoDb, testcontainers::runners::AsyncRunner};

    #[tokio::test]
    async fn dynamodb_local_create_table() {
        let _ = pretty_env_logger::try_init();
        let node = DynamoDb.start().await;
        let host_ip = node.get_host().await;
        let host_port = node.get_host_port_ipv4(8000).await;

        let table_name = "books".to_string();

        let key_schema = KeySchemaElement::builder()
            .attribute_name("title".to_string())
            .key_type(KeyType::Hash)
            .build()
            .unwrap();

        let attribute_def = AttributeDefinition::builder()
            .attribute_name("title".to_string())
            .attribute_type(ScalarAttributeType::S)
            .build()
            .unwrap();

        let provisioned_throughput = ProvisionedThroughput::builder()
            .read_capacity_units(10)
            .write_capacity_units(5)
            .build()
            .unwrap();

        let dynamodb = build_dynamodb_client(host_ip, host_port).await;
        let create_table_result = dynamodb
            .create_table()
            .table_name(table_name)
            .key_schema(key_schema)
            .attribute_definitions(attribute_def)
            .provisioned_throughput(provisioned_throughput)
            .send()
            .await;
        assert!(create_table_result.is_ok());

        let req = dynamodb.list_tables().limit(10);
        let list_tables_result = req.send().await.unwrap();

        assert_eq!(list_tables_result.table_names().len(), 1);
    }

    async fn build_dynamodb_client(host_ip: IpAddr, host_port: u16) -> Client {
        let endpoint_uri = format!("http://{host_ip}:{host_port}");
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let creds = Credentials::new("fakeKey", "fakeSecret", None, None, "test");

        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint_uri)
            .credentials_provider(creds)
            .load()
            .await;

        Client::new(&shared_config)
    }
}
