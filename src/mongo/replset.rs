use testcontainers::{
    core::{ExecCommand, WaitFor},
    Image, ImageArgs,
};

use super::{NAME, TAG};

#[derive(Debug, Clone)]
pub struct MongoReplSet {
    name: String,
    tag: String,
}

#[derive(Debug, Clone, Default)]
pub struct ReplSetArgs;

impl ImageArgs for ReplSetArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(vec!["--replSet".to_string(), "rs".to_string()].into_iter())
    }
}

impl Default for MongoReplSet {
    fn default() -> Self {
        Self {
            name: NAME.to_owned(),
            tag: TAG.to_owned(),
        }
    }
}

impl MongoReplSet {
    pub fn with_name(mut self, name: impl AsRef<str>) -> Self {
        self.name = name.as_ref().to_string();
        self
    }

    pub fn with_tag(mut self, tag: impl AsRef<str>) -> Self {
        self.tag = tag.as_ref().to_string();
        self
    }
}

impl Image for MongoReplSet {
    type Args = ReplSetArgs;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections")]
    }

    fn exec_after_start(
        &self,
        _: testcontainers::core::ContainerState,
    ) -> Result<Vec<ExecCommand>, testcontainers::TestcontainersError> {
        Ok(vec![ExecCommand::new(vec![
            "mongosh".to_string(),
            "--quiet".to_string(),
            "--eval".to_string(),
            "'rs.initiate()'".to_string(),
        ])
        .with_cmd_ready_condition(WaitFor::message_on_stdout(
            "Using a default configuration for the set",
        ))
        // Wait for the replica set to be ready
        // this is a workaround for the fact that the replica set is not ready immediately
        // without this, a inmediate connection to the replica set will fail
        .with_container_ready_conditions(vec![WaitFor::seconds(2)])])
    }
}

#[cfg(test)]
mod tests {
    use mongodb::*;
    use testcontainers::runners::AsyncRunner;

    use crate::mongo::{self, NAME_TAG_VARIANTS};

    #[tokio::test]
    async fn mongo_repl_set_fetch_document() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = mongo::MongoReplSet::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(27017).await?;
        let url = format!("mongodb://{host_ip}:{host_port}/?directConnection=true",);

        assert_mongo_works_with_transaction(&url).await?;

        Ok(())
    }

    #[tokio::test]
    async fn mongo_repl_set_works_on_variants() -> Result<(), Box<dyn std::error::Error + 'static>>
    {
        let _ = pretty_env_logger::try_init();
        for (name, tag) in NAME_TAG_VARIANTS {
            let node = mongo::MongoReplSet::default()
                .with_name(name)
                .with_tag(tag)
                .start()
                .await?;
            let host_ip = node.get_host().await?;
            let host_port = node.get_host_port_ipv4(27017).await?;
            let url = format!("mongodb://{host_ip}:{host_port}/?directConnection=true",);

            assert_mongo_works_with_transaction(&url).await?;
        }

        Ok(())
    }
    async fn assert_mongo_works_with_transaction(
        url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client: Client = Client::with_uri_str(url).await?;
        let db = client.database("some_db");
        let coll = db.collection("some-coll");

        let mut session = client.start_session(None).await?;
        session.start_transaction(None).await?;

        let insert_one_result = coll
            .insert_one_with_session(bson::doc! { "x": 42 }, None, &mut session)
            .await?;
        assert!(!insert_one_result
            .inserted_id
            .as_object_id()
            .unwrap()
            .to_hex()
            .is_empty());
        session.commit_transaction().await?;

        let find_one_result: bson::Document = coll
            .find_one(bson::doc! { "x": 42 }, None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(42, find_one_result.get_i32("x").unwrap());

        Ok(())
    }
}
