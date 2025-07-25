# Contributing to testcontainers-rs-modules

First of all, thank you for contributing to testcontainers-rs-modules-community. All contributions are welcome!

## Getting started

### The quick-wins!

- Consider giving a star to this repo and become one of our [stargazers](https://github.com/testcontainers/testcontainers-rs-modules-community/stargazers)
- Consider joining our [Rust channel](https://testcontainers.slack.com/archives/C048EPGRCER) within the [TestContainers Slack Workspace](https://testcontainers.slack.com/)

### Reporting bugs

- Check if there is an [existing issue](https://github.com/testcontainers/testcontainers-rs-modules-community/issues) first. 
- You may also want to check bugs in [testcontainers-rs library](https://github.com/testcontainers/testcontainers-rs/issues) when applicable
- When in doubt whether you've found new issue/bug or not, consider discussing it with our community on Slack

### Requesting new modules

- Check if there is an [existing issue](https://github.com/testcontainers/testcontainers-rs-modules-community/issues), since your request might be tracked there.
- Feel free to reach out to our community to clarify your use case!

## Code Contributions

### Setting up local development

- Ensure you have an [up-to-date Rust toolchain](https://rustup.rs/), with `clippy` and `rustfmt` components installed
- Install the [cargo-hack](https://github.com/taiki-e/cargo-hack) subcommand (recommended)
- Fork this repository
- Optionally, if you need to run `Oracle` tests, you must setup `Oracle Client`, as indicated in the documentation of the [Rust-oracle](https://docs.rs/oracle/latest/oracle/) crate (note that ARM architecture is not supported, as there is no Oracle Database Free port for ARM chips)

### Working with existing modules

- When extending/changing existing API, ensure that any new ones are tracked on [rustdocs](https://docs.rs/testcontainers-modules/latest/testcontainers_modules/)
- When fixing an issue, consider providing a new test case for it

### Adding new modules or changing existing ones

Before adding a new module, it's recommended reviewing the
[testcontainers-rs library](https://github.com/testcontainers/testcontainers-rs)
along with existing modules that are built on top of it! In addition, pay attention to our project conventions : public APIs 
are exposed through `mod.rs` files and conditionally compiled as Cargo features. 

- Ensure you have a proper feature configuration on `Cargo.toml` when adding a new module
- Ensure you have declared pinned versions of Docker image tags for your module
- Consider providing a `Default` trait implementation for your `TestContainer` struct when applicable
- Consider also using the `Builder` pattern when your `TestContainer` accepts several different env vars!
- Ensure you have added proper `rustdocs` reflecting the image reference (e.g. to [docker-hub](hub.docker.com)) and examples of how to use your APIs.
- Ensure you have added tests exercising your module
- Consider also providing an example on how to use your module from an app

### Raising Pull Requests

- Ensure you'll have a green build on your commits, for instance running some of our CI checks

```bash
cargo fmt --all -- --check
cargo clippy --all-features
cargo hack test --each-feature --clean-per-run 
```
- Consider following [conventional commits](https://julien.ponge.org/blog/the-power-of-conventional-commits/) when adding commits (recommended)
- Raise your PR 🔥
- Ensure you follow the [conventional commits](https://julien.ponge.org/blog/the-power-of-conventional-commits/) in your Pull Request title
- Don't forget to [link an existing issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue#linking-a-pull-request-to-an-issue-using-a-keyword) when applicable (fixing it or just mentioning it)

## License

Please note that all code contributed by you will follow the [MIT license](http://opensource.org/licenses/MIT),
without any additional terms.
