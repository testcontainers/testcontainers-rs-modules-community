use testcontainers::{core::WaitFor, Image};

const NAME: &str = "redis/redis-stack-server";
const TAG: &str = "7.2.0-v8";

/// Module to work with [`Redis Stack`] inside of tests.
///
/// Starts an instance of Redis Stack based on the official [`Redis Stack docker image`].
///
/// By default Redis is exposed on Port 6379 ([`REDIS_PORT`]) and has no access control. Please refer to the [`Redis reference guide`] for more informations on how to interact with the API.
///
/// # Example
/// ```
/// use redis::JsonCommands;
/// use serde_json::json;
/// use testcontainers_modules::{testcontainers::runners::SyncRunner, redis::{RedisStack, REDIS_PORT}};
///
/// let redis_instance = RedisStack.start().unwrap();
/// let host_ip = redis_instance.get_host().unwrap();
/// let host_port = redis_instance.get_host_port_ipv4(REDIS_PORT).unwrap();
///
/// let url = format!("redis://{host_ip}:{host_port}");
/// let client = redis::Client::open(url.as_ref()).unwrap();
/// let mut con = client.get_connection().unwrap();
///
/// con.json_set::<_,_,_,()>("my_key", "$", &json!({ "number": 42 })).unwrap();
/// let result: String = con.json_get("my_key", "$..number").unwrap();
/// ```
///
/// [`Redis Stack`]: https://redis.io/docs/about/about-stack/
/// [`Redis Stack docker image`]: https://hub.docker.com/r/redis/redis-stack-server
/// [`Redis reference guide`]: https://redis.io/docs/interact/
/// [`REDIS_PORT`]: super::REDIS_PORT
#[derive(Debug, Default, Clone)]
pub struct RedisStack;

impl Image for RedisStack {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}

#[cfg(test)]
mod tests {
    use redis::JsonCommands;
    use serde_json::json;

    use crate::{
        redis::{RedisStack, REDIS_PORT},
        testcontainers::runners::SyncRunner,
    };

    #[test]
    fn redis_fetch_an_integer_in_json() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = RedisStack.start()?;
        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(REDIS_PORT)?;
        let url = format!("redis://{host_ip}:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        assert_eq!(
            con.json_set("my_key", "$", &json!({ "number": 42 })),
            Ok(true)
        );
        let result: String = con.json_get("my_key", "$..number").unwrap();
        assert_eq!("[42]", result);
        Ok(())
    }
}
