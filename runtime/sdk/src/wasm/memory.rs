use super::raw;

pub fn memory_read(key: &str) -> Vec<u8> {
    let key_len = key.len() as i64;
    let mut key = key.to_string().into_bytes();
    let value_len = unsafe { raw::memory_read_length(key.as_mut_ptr(), key_len) };
    let mut result_data_ptr = vec![0; value_len as usize];
    unsafe {
        raw::memory_read(key.as_mut_ptr(), key_len, result_data_ptr.as_mut_ptr(), value_len);
    }
    result_data_ptr
}

// TODO: Value could be cleaned up to a generic that implements our ToBytes
// trait :)
pub fn memory_write(key: &str, mut value: Vec<u8>) {
    let key_len = key.len() as i64;
    let mut key = key.to_string().into_bytes();
    let value_len = value.len() as i64;
    unsafe {
        raw::memory_write(key.as_mut_ptr(), key_len, value.as_mut_ptr(), value_len);
    }
}

pub fn shared_memory_get(key: &str) -> Vec<u8> {
    let key_len = key.len() as i64;
    let mut key = key.to_string().into_bytes();
    let value_len = unsafe { raw::shared_memory_read_length(key.as_mut_ptr(), key_len) };
    let mut result_data_ptr = vec![0; value_len as usize];
    unsafe {
        raw::shared_memory_read(key.as_mut_ptr(), key_len, result_data_ptr.as_mut_ptr(), value_len);
    }
    result_data_ptr
}

// TODO: Value could be cleaned up to a generic that implements our ToBytes
// trait :)
pub fn shared_memory_set(key: &str, mut value: Vec<u8>) {
    let key_len = key.len() as i64;
    let mut key = key.to_string().into_bytes();
    let value_len = value.len() as i64;
    unsafe {
        raw::shared_memory_write(key.as_mut_ptr(), key_len, value.as_mut_ptr(), value_len);
    }
}
