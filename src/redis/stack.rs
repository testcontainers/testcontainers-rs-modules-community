use testcontainers::{core::WaitFor, Image};

const NAME: &str = "redis/redis-stack-server";
const TAG: &str = "7.2.0-v8";

#[derive(Debug)]
pub struct RedisStack;

impl Image for RedisStack {
    type Args = ();

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        TAG.to_owned()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}

#[cfg(test)]
mod tests {
    use redis::JsonCommands;
    use serde_json::json;
    use testcontainers::clients;

    use crate::redis::RedisStack;

    #[test]
    fn redis_fetch_an_integer_in_json() {
        let _ = pretty_env_logger::try_init();
        let docker = clients::Cli::default();
        let node = docker.run(RedisStack);
        let host_port = node.get_host_port_ipv4(6379);
        let url = format!("redis://127.0.0.1:{host_port}");

        let client = redis::Client::open(url.as_ref()).unwrap();
        let mut con = client.get_connection().unwrap();

        assert_eq!(
            con.json_set("my_key", "$", &json!({ "number": 42 })),
            Ok(true)
        );
        let result: String = con.json_get("my_key", "$..number").unwrap();
        assert_eq!("[42]", result);
    }
}
