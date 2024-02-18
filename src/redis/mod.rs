mod stack;
mod standalone;

pub const REDIS_PORT: u16 = 6379;

pub use stack::RedisStack;
pub use standalone::Redis;
