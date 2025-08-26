use testcontainers::{
    core::{CmdWaitFor, ExecCommand, WaitFor},
    Image,
};

const NAME: &str = "mongo";
const TAG: &str = "5.0.6";

/// Type of MongoDB instance to create.
#[derive(Default, Debug, Clone)]
enum InstanceKind {
    /// A standalone MongoDB instance (default).
    #[default]
    Standalone,
    /// A MongoDB replica set instance.
    ReplSet,
}

/// Module to work with [`MongoDB`] inside of tests.
///
/// Starts an instance of MongoDB based on the official [`MongoDB docker image`].
///
/// This module supports both standalone and replica set configurations.
/// The container exposes port `27017` by default.
///
/// # Example
/// ```
/// use testcontainers_modules::{mongo::Mongo, testcontainers::runners::SyncRunner};
///
/// let mongo_instance = Mongo::default().start().unwrap();
/// let host = mongo_instance.get_host().unwrap();
/// let port = mongo_instance.get_host_port_ipv4(27017).unwrap();
///
/// // Connect to MongoDB at mongodb://{host}:{port}
/// ```
///
/// [`MongoDB`]: https://www.mongodb.com/
/// [`MongoDB docker image`]: https://hub.docker.com/_/mongo
#[derive(Default, Debug, Clone)]
pub struct Mongo {
    kind: InstanceKind,
}

impl Mongo {
    /// Creates a new standalone MongoDB instance.
    ///
    /// This is equivalent to using `Mongo::default()`.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::mongo::Mongo;
    ///
    /// let mongo = Mongo::new();
    /// ```
    pub fn new() -> Self {
        Self {
            kind: InstanceKind::Standalone,
        }
    }
    /// Creates a new MongoDB replica set instance.
    ///
    /// This configures MongoDB to run as a replica set, which is useful for testing
    /// replica set specific features like transactions.
    ///
    /// # Example
    /// ```
    /// use testcontainers_modules::mongo::Mongo;
    ///
    /// let mongo = Mongo::repl_set();
    /// ```
    pub fn repl_set() -> Self {
        Self {
            kind: InstanceKind::ReplSet,
        }
    }
}

impl Image for Mongo {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections")]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<std::borrow::Cow<'_, str>>> {
        match self.kind {
            InstanceKind::Standalone => Vec::<String>::new(),
            InstanceKind::ReplSet => vec!["--replSet".to_string(), "rs".to_string()],
        }
    }

    fn exec_after_start(
        &self,
        _: testcontainers::core::ContainerState,
    ) -> Result<Vec<ExecCommand>, testcontainers::TestcontainersError> {
        match self.kind {
            InstanceKind::Standalone => Ok(Default::default()),
            InstanceKind::ReplSet => Ok(vec![ExecCommand::new(vec![
                "mongosh".to_string(),
                "--quiet".to_string(),
                "--eval".to_string(),
                "'rs.initiate()'".to_string(),
            ])
            .with_cmd_ready_condition(CmdWaitFor::message_on_stdout(
                "Using a default configuration for the set",
            ))
            .with_container_ready_conditions(vec![WaitFor::message_on_stdout(
                "Rebuilding PrimaryOnlyService due to stepUp",
            )])]),
        }
    }
}

#[cfg(test)]
mod tests {
    use mongodb::*;
    use testcontainers::{core::IntoContainerPort, runners::AsyncRunner};

    use crate::mongo;

    #[tokio::test]
    async fn mongo_fetch_document() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = mongo::Mongo::default().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(27017.tcp()).await?;
        let url = format!("mongodb://{host_ip}:{host_port}/");

        let client: Client = Client::with_uri_str(&url).await.unwrap();
        let db = client.database("some_db");
        let coll = db.collection("some_coll");

        let insert_one_result = coll.insert_one(bson::doc! { "x": 42 }).await.unwrap();
        assert!(!insert_one_result
            .inserted_id
            .as_object_id()
            .unwrap()
            .to_hex()
            .is_empty());

        let find_one_result: bson::Document = coll
            .find_one(bson::doc! { "x": 42 })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(42, find_one_result.get_i32("x").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn mongo_repl_set_fetch_document() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = mongo::Mongo::repl_set().start().await?;
        let host_ip = node.get_host().await?;
        let host_port = node.get_host_port_ipv4(27017).await?;
        let url = format!("mongodb://{host_ip}:{host_port}/?directConnection=true",);

        let client: Client = Client::with_uri_str(url).await?;
        let db = client.database("some_db");
        let coll = db.collection("some-coll");

        let mut session = client.start_session().await?;
        session.start_transaction().await?;

        let insert_one_result = coll
            .insert_one(bson::doc! { "x": 42 })
            .session(&mut session)
            .await?;
        assert!(!insert_one_result
            .inserted_id
            .as_object_id()
            .unwrap()
            .to_hex()
            .is_empty());
        session.commit_transaction().await?;

        let find_one_result: bson::Document = coll
            .find_one(bson::doc! { "x": 42 })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(42, find_one_result.get_i32("x").unwrap());
        Ok(())
    }
}
