extern "C" {
    pub fn promise_then(action_data_offset: *const u8, action_data_length: i32);
    pub fn promise_status_length(promise_index: i32) -> i64;
    pub fn promise_status_write(promise_index: i32, result_data_offset: *const u8, result_data_length: i64);
    pub fn memory_read(key: *const u8, key_length: i64, result_data_ptr: *const u8, result_data_length: i64);
    pub fn memory_read_length(key: *const u8, key_length: i64) -> i64;
    pub fn memory_write(key: *const u8, key_length: i64, value: *const u8, value_length: i64);
    pub fn shared_memory_read(key: *const u8, key_length: i64, result_data_ptr: *const u8, result_data_length: i64);
    pub fn shared_memory_read_length(key: *const u8, key_length: i64) -> i64;
    pub fn shared_memory_write(key: *const u8, key_length: i64, value: *const u8, value_length: i64);
    pub fn execution_result(result: *const u8, result_length: i32);
    pub fn _log(
        level: *const u8,
        level_len: i32,
        msg: *const u8,
        msg_len: i64,
        line_info: *const u8,
        line_info_len: i64,
    );
    pub fn bn254_verify(
        message: *const u8,
        message_length: i64,
        signature: *const u8,
        signature_length: i64,
        public_key: *const u8,
        public_key_length: i64,
    ) -> u8;
    pub fn bn254_sign(message: *const u8, message_length: i64, result_data_ptr: *const u8, result_data_length: i64);
}
