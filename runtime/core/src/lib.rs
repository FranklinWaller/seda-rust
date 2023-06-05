//! WASI compatible WASM VM

mod vm_call_data;
pub use vm_call_data::*;

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

mod runtime_context;
pub use runtime_context::*;

mod vm_import_types;
pub use vm_import_types::*;

#[cfg(test)]
#[path = ""]
mod test {
    mod test_host;
    pub(crate) use test_host::*;

    mod runtime_test;
}
