#[derive(Debug, Clone)]
pub struct VmCallData {
    /// Name of the binary, ex. "consensus", "fisherman", etc
    pub program_name: String,

    // The function we need to execute, defaults to the WASI default ("_start")
    pub start_func: Option<String>,

    /// Arguments to pass to the WASM binary
    pub args: Vec<String>,

    pub debug: bool,
}
