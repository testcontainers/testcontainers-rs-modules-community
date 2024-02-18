mod standalone;
mod stack;

pub const REDIS_PORT: u16 = 6379;

pub use standalone::Redis;
pub use stack::RedisStack;
