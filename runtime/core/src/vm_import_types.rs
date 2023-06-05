use lazy_static::lazy_static;

lazy_static! {
    pub static ref CORE_IMPORTS: Vec<String> = {
        vec![
            "shared_memory_contains_key",
            "shared_memory_read",
            "shared_memory_read_length",
            "shared_memory_write",
            "execution_result",
            "_log",
            "bn254_verify",
            "bn254_sign",
            "http_fetch",
            "chain_view",
            "chain_call",
            "db_set",
            "db_get",
            "p2p_broadcast",
            "trigger_event",
            "call_result_write",
            "call_result_length",

            // WASI Imports
            "args_get",
            "args_sizes_get",
            "fd_write", // For logging to dev/stdout
            "random_get",
            "environ_get",
            "environ_sizes_get",
            "proc_exit",
        ]
        .iter()
        .map(|import| import.to_string())
        .collect()
    };
    pub static ref DATAREQUEST_IMPORTS: Vec<String> = {
        vec!["execution_result", "bn254_verify", "http_fetch", "call_result_write", "args_get", "args_sizes_get"]
            .iter()
            .map(|import| import.to_string())
            .collect()
    };
    pub static ref AGGREGATION_IMPORTS: Vec<String> = {
        vec!["execution_result", "bn254_verify", "call_result_write", "args_get", "args_sizes_get"]
            .iter()
            .map(|import| import.to_string())
            .collect()
    };
}
