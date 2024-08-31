#[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
/// **Oracle Database Free** (relational database) testcontainer
pub mod free;
