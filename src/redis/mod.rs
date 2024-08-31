mod stack;
mod standalone;

/// Port that the [`Redis`] container has internally
/// Can be rebound externally via [`testcontainers::core::ImageExt::with_mapped_port`]
///
/// [`Redis`]: https://redis.io/
pub const REDIS_PORT: u16 = 6379;

pub use stack::RedisStack;
pub use standalone::Redis;
