use std::{borrow::Cow, collections::HashMap};

use testcontainers::{
    core::{wait::HttpWaitStrategy, ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "getmeili/meilisearch";
const TAG: &str = "v1.8.3";
const MEILISEARCH_PORT: ContainerPort = ContainerPort::Tcp(7700);

/// Module to work with [`Meilisearch`] inside of tests.
///
/// Starts an instance of Meilisearch.
/// This module is based on the official [`Meilisearch docker image`] documented in the [`Meilisearch docker docs`].
///
/// # Example
/// ```
/// use testcontainers_modules::{meilisearch, testcontainers::runners::SyncRunner};
///
/// let meilisearch_instance = meilisearch::Meilisearch::default().start().unwrap();
///
/// let dashboard = format!(
///     "http://{}:{}",
///     meilisearch_instance.get_host().unwrap(),
///     meilisearch_instance.get_host_port_ipv4(7700).unwrap()
/// );
/// ```
///
/// [`Meilisearch`]: https://www.meilisearch.com/
/// [`Meilisearch docker docs`]: https://www.meilisearch.com/docs/guides/misc/docker
/// [`Meilisearch docker image`]: https://hub.docker.com/_/getmeili/meilisearch
#[derive(Debug, Clone)]
pub struct Meilisearch {
    env_vars: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Environment {
    Production,
    Development,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
    Off,
}

impl Meilisearch {
    /// Sets the `MASTER_KEY` for the [`Meilisearch`] instance.
    /// Default `MASTER_KEY` is `None` if not overridden by this function
    ///
    /// See the [official docs for this option](https://www.meilisearch.com/docs/learn/configuration/instance_options#master-key)
    pub fn with_master_key(mut self, master_key: &str) -> Self {
        self.env_vars
            .insert("MEILI_MASTER_KEY".to_owned(), master_key.to_owned());
        self
    }

    /// Configures analytics for the [`Meilisearch`] instance.
    /// Default is `false` if not overridden by this function
    /// This default differs from the dockerfile as we expect tests not to be good analytics.
    ///
    /// See the [official docs for this option](https://www.meilisearch.com/docs/learn/configuration/instance_options#log-level)
    pub fn with_analytics(mut self, enabled: bool) -> Self {
        if enabled {
            self.env_vars.remove("MEILI_NO_ANALYTICS");
        } else {
            self.env_vars
                .insert("MEILI_NO_ANALYTICS".to_owned(), "true".to_owned());
        }
        self
    }

    /// Sets the environment of the [`Meilisearch`] instance.
    /// Default is [Environment::Development] if not overridden by this function.
    /// Setting it to [Environment::Production] requires authentication via [Meilisearch::with_master_key]
    ///
    /// See the [official docs for this option](https://www.meilisearch.com/docs/learn/configuration/instance_options#environment)
    pub fn with_environment(mut self, environment: Environment) -> Self {
        let env = match environment {
            Environment::Production => "production".to_owned(),
            Environment::Development => "development".to_owned(),
        };
        self.env_vars.insert("MEILI_ENV".to_owned(), env);
        self
    }

    /// Sets the log level of the [`Meilisearch`] instance.
    /// Default is [LogLevel::Info] if not overridden by this function.
    ///
    /// See the [official docs for this option](https://www.meilisearch.com/docs/learn/configuration/instance_options#disable-analytics)
    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        let level = match level {
            LogLevel::Error => "ERROR".to_owned(),
            LogLevel::Warn => "WARN".to_owned(),
            LogLevel::Info => "INFO".to_owned(),
            LogLevel::Debug => "DEBUG".to_owned(),
            LogLevel::Trace => "TRACE".to_owned(),
            LogLevel::Off => "OFF".to_owned(),
        };
        self.env_vars.insert("MEILI_LOG_LEVEL".to_owned(), level);
        self
    }
}

impl Default for Meilisearch {
    /**
     * Starts an instance
     * - in `development` mode (see [Meilisearch::with_environment] to change this)
     * - without `MASTER_KEY` being set (see [Meilisearch::with_master_key] to change this)
     * - with Analytics disabled (see [Meilisearch::with_analytics] to change this)
     */
    fn default() -> Self {
        let mut env_vars = HashMap::new();
        env_vars.insert("MEILI_NO_ANALYTICS".to_owned(), "true".to_owned());
        Self { env_vars }
    }
}

impl Image for Meilisearch {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // the container does allow for turning off logging entirely and does not have a healthcheck
        // => using the `/health` endpoint is the best strategy
        vec![WaitFor::http(
            HttpWaitStrategy::new("/health")
                .with_expected_status_code(200_u16)
                .with_body(r#"{ "status": "available" }"#.as_bytes()),
        )]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        &self.env_vars
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[MEILISEARCH_PORT]
    }
}

#[cfg(test)]
mod tests {
    use meilisearch_sdk::{client::Client, indexes::Index};
    use serde::{Deserialize, Serialize};
    use testcontainers::{runners::AsyncRunner, ImageExt};

    use super::*;
    #[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
    struct Movie {
        id: i64,
        title: String,
    }

    impl From<(i64, &str)> for Movie {
        fn from((id, title): (i64, &str)) -> Self {
            Self {
                id,
                title: title.to_owned(),
            }
        }
    }

    impl Movie {
        fn examples() -> Vec<Self> {
            vec![
                Movie::from((1, "The Shawshank Redemption")),
                Movie::from((2, "The Godfather")),
                Movie::from((3, "The Dark Knight")),
                Movie::from((4, "Pulp Fiction")),
                Movie::from((5, "The Lord of the Rings: The Return of the King")),
                Movie::from((6, "Forrest Gump")),
                Movie::from((7, "Inception")),
                Movie::from((8, "Fight Club")),
                Movie::from((9, "The Matrix")),
                Movie::from((10, "Goodfellas")),
            ]
        }
        async fn get_index_with_loaded_examples(
            client: &Client,
        ) -> Result<Index, Box<dyn std::error::Error + 'static>> {
            let task = client
                .create_index("movies", None)
                .await?
                .wait_for_completion(client, None, None)
                .await?;
            let movies = task.try_make_index(client).unwrap();
            assert_eq!(movies.as_ref(), "movies");
            movies
                .add_documents(&Movie::examples(), Some("id"))
                .await?
                .wait_for_completion(client, None, None)
                .await?;
            Ok(movies)
        }
    }

    #[tokio::test]
    async fn meilisearch_noauth() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let meilisearch_image = Meilisearch::default();
        let node = meilisearch_image.start().await?;

        let connection_string = &format!(
            "http://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(7700).await?,
        );
        let auth: Option<String> = None; // not currently possible to type-infer String or that it is not nessesary
        let client = Client::new(connection_string, auth).unwrap();

        // healthcheck
        let res = client.health().await.unwrap();
        assert_eq!(res.status, "available");

        // insert documents and search for them
        let movies = Movie::get_index_with_loaded_examples(&client).await?;
        let res = movies
            .search()
            .with_query("Dark Knig")
            .with_limit(5)
            .execute::<Movie>()
            .await?;
        let results = res
            .hits
            .into_iter()
            .map(|r| r.result)
            .collect::<Vec<Movie>>();
        assert_eq!(
            results,
            vec![Movie {
                id: 3,
                title: String::from("The Dark Knight")
            }]
        );
        assert_eq!(res.estimated_total_hits, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn meilisearch_custom_version() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let master_key = "secret master key".to_owned();
        let meilisearch_image = Meilisearch::default()
            .with_master_key(&master_key)
            .with_tag("v1.0");
        let node = meilisearch_image.start().await?;

        let connection_string = &format!(
            "http://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(7700).await?,
        );
        let client = Client::new(connection_string, Some(master_key)).unwrap();

        // insert documents and search for it
        let movies = Movie::get_index_with_loaded_examples(&client).await?;
        let res = movies
            .search()
            .with_query("Dark Knig")
            .execute::<Movie>()
            .await?;
        let result_ids = res
            .hits
            .into_iter()
            .map(|r| r.result.id)
            .collect::<Vec<i64>>();
        assert_eq!(result_ids, vec![3]);
        Ok(())
    }

    #[tokio::test]
    async fn meilisearch_without_logging_in_production_environment(
    ) -> Result<(), Box<dyn std::error::Error + 'static>> {
        let master_key = "secret master key".to_owned();
        let meilisearch_image = Meilisearch::default()
            .with_environment(Environment::Production)
            .with_log_level(LogLevel::Off)
            .with_master_key(&master_key);
        let node = meilisearch_image.start().await?;

        let connection_string = &format!(
            "http://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(7700).await?,
        );
        let client = Client::new(connection_string, Some(master_key)).unwrap();

        // insert documents and search for it
        let movies = Movie::get_index_with_loaded_examples(&client).await?;
        let res = movies
            .search()
            .with_query("Dark Knig")
            .execute::<Movie>()
            .await?;
        let result_ids = res
            .hits
            .into_iter()
            .map(|r| r.result.id)
            .collect::<Vec<i64>>();
        assert_eq!(result_ids, vec![3]);
        Ok(())
    }
}
