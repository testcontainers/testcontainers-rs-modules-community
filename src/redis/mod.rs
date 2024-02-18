mod default;
mod stack;

pub const REDIS_PORT: u16 = 6379;

pub use default::Redis;
pub use stack::RedisStack;
