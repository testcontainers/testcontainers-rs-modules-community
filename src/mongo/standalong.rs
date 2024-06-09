use testcontainers::{core::WaitFor, Image};

use super::{NAME, TAG};

#[derive(Debug, Clone)]
pub struct Mongo {
    name: String,
    tag: String,
}

impl Default for Mongo {
    fn default() -> Self {
        Self {
            name: NAME.to_owned(),
            tag: TAG.to_owned(),
        }
    }
}

impl Mongo {
    pub fn with_name(mut self, name: impl AsRef<str>) -> Self {
        self.name = name.as_ref().to_string();
        self
    }

    pub fn with_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.tag = tag.as_ref().to_string();
        self
    }
}

impl Image for Mongo {
    type Args = ();

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections")]
    }
}

#[cfg(test)]
mod tests {
    use mongodb::*;
    use testcontainers::runners::AsyncRunner;

    use crate::mongo::{self, NAME_TAG_VARIANTS};

    #[tokio::test]
    async fn mongo_fetch_document() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = mongo::Mongo::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(27017).await?;
        let url = format!("mongodb://{host_ip}:{host_port}/?directConnection=true",);

        assert_mongo_works(&url).await;
        Ok(())
    }

    #[tokio::test]
    async fn mongo_works_on_variants() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        for (name, tag) in NAME_TAG_VARIANTS {
            let node = mongo::Mongo::default()
                .with_name(name)
                .with_tag(tag)
                .start()
                .await?;
            let host_ip = node.get_host().await?;
            let host_port = node.get_host_port_ipv4(27017).await?;
            let url = format!("mongodb://{host_ip}:{host_port}/?directConnection=true",);

            assert_mongo_works(&url).await;
        }

        Ok(())
    }

    async fn assert_mongo_works(url: &str) {
        let client: Client = Client::with_uri_str(url).await.unwrap();
        let db = client.database("some_db");
        let coll = db.collection("some-coll");
        let insert_one_result = coll.insert_one(bson::doc! { "x": 42 }, None).await.unwrap();
        assert!(!insert_one_result
            .inserted_id
            .as_object_id()
            .unwrap()
            .to_hex()
            .is_empty());

        let find_one_result: bson::Document = coll
            .find_one(bson::doc! { "x": 42 }, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(42, find_one_result.get_i32("x").unwrap());
    }
}
