# testcontainers-modules

![Continuous Integration](https://github.com/testcontainers/testcontainers-rs-modules-community/workflows/Continuous%20Integration/badge.svg?branch=main)
[![Crates.io](https://img.shields.io/crates/v/testcontainers-modules.svg)](https://crates.io/crates/testcontainers-modules)
[![Docs.rs](https://docs.rs/testcontainers-modules/badge.svg)](https://docs.rs/testcontainers-modules)

Community maintained modules for [testcontainers]

Provides modules to use for testing components in accordance with [testcontainers-rs].
Every module is treated as a feature inside this crate.

## Usage

1. Depend on [testcontainers-modules] with necessary features (e.g `postgres`, `minio` and etc)
2. Then start using the modules inside your tests.

**Note**: you don't need to explicitly depend on `testcontainers` as it's re-exported dependency of `testcontainers-modules` with aligned version between these crates.
For example: 
```rust
use testcontainers_modules::testcontainers::RunnableImage;
```

You can also see [examples](https://github.com/testcontainers/testcontainers-rs-modules-community/tree/main/examples) for more details. 

### How to override module defaults (version, tag, ENV-variables)
Just use [RunnableImage](https://docs.rs/testcontainers/latest/testcontainers/core/struct.RunnableImage.html):
```rust
use testcontainers_modules::{
    redis::Redis,
    testcontainers::RunnableImage
};

/// Create a Redis module with `6.2-alpine` tag and custom password
fn create_redis() -> RunnableImage<Redis> {
    RunnableImage::from(Redis::default())
        .with_tag("6.2-alpine")
        .with_env_var(("REDIS_PASSWORD", "my_secret_password"))
}
```


## License

- MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[testcontainers-rs]: https://github.com/testcontainers/testcontainers-rs
[testcontainers]: https://crates.io/crates/testcontainers
[testcontainers-modules]: https://crates.io/crates/testcontainers-modules
[LICENSE]: https://github.com/testcontainers/testcontainers-rs-modules-community/blob/main/LICENSE
