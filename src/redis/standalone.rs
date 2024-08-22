use testcontainers::{core::WaitFor, Image};

const NAME: &str = "redis";
const TAG: &str = "5.0";

/// Module to work with [`Redis`] inside of tests.
///
/// Starts an instance of Redis based on the official [`Redis docker image`].
///
/// By default Redis is exposed on Port 6379 ([`REDIS_PORT`]) and has no access control. Please refer to the [`Redis reference guide`] for more informations on how to interact with the API.
///
/// # Example
/// ```
/// use redis::Commands;
/// use testcontainers_modules::{
///     redis::{Redis, REDIS_PORT},
///     testcontainers::runners::SyncRunner,
/// };
///
/// let redis_instance = Redis::default().start().unwrap();
/// let host_ip = redis_instance.get_host().unwrap();
/// let host_port = redis_instance.get_host_port_ipv4(REDIS_PORT).unwrap();
///
/// let url = format!("redis://{host_ip}:{host_port}");
/// let client = redis::Client::open(url.as_ref()).unwrap();
/// let mut con = client.get_connection().unwrap();
///
/// con.set::<_, _, ()>("my_key", 42).unwrap();
/// let result: i64 = con.get("my_key").unwrap();
/// ```
///
/// [`Redis`]: https://redis.io/
/// [`Redis docker image`]: https://hub.docker.com/_/redis
/// [`Redis reference guide`]: https://redis.io/docs/interact/
/// [`REDIS_PORT`]: super::REDIS_PORT
#[derive(Debug, Default, Clone)]
pub struct Redis;

impl Image for Redis {
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
    use redis::Commands;

    use crate::{redis::Redis, testcontainers::runners::SyncRunner};

    #[test]
    fn redis_fetch_an_integer() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let node = Redis.start()?;
        let host_ip = node.get_host()?;
        let host_port = node.get_host_port_ipv4(6379)?;
        let url = format!("redis://{host_ip}:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        con.set::<_, _, ()>("my_key", 42).unwrap();
        let result: i64 = con.get("my_key").unwrap();
        assert_eq!(42, result);
        Ok(())
    }
}
