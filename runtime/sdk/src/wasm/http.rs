use super::Promise;
use crate::{HttpAction, PromiseAction};

pub fn http_fetch(url: &str) -> Promise {
    Promise::new(PromiseAction::Http(HttpAction { url: url.into() }))
}

pub fn http_fetch_new(url: &str) -> String {
    let http_action = HttpAction { url: url.to_string() };

    let action = serde_json::to_string(&http_action).unwrap();
    let result_length = unsafe { super::raw::http_fetch(action.as_ptr(), action.len() as u32) };
    let mut result_data_ptr = vec![0; result_length as usize];

    unsafe {
        super::raw::call_result_write(result_data_ptr.as_mut_ptr(), result_length);
    }

    String::from_utf8(result_data_ptr).unwrap()
}
