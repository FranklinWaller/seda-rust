//! WASI compatible WASM VM

mod config;
pub use config::*;

mod context;
pub use context::*;

mod errors;
pub use errors::*;

mod host_adapter;
pub use host_adapter::*;

pub(crate) mod imports;

mod runtime;
pub use runtime::*;

mod storage;
pub use storage::*;

mod vm_result;
pub use vm_result::*;

#[cfg(test)]
#[path = ""]
mod test {
    mod test_host;
    pub(crate) use test_host::*;

    mod runtime_test;
}
