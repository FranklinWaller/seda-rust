use super::Promise;
use crate::{HttpAction, PromiseAction};

pub fn http_fetch(url: &str) -> Promise {
    Promise::new(PromiseAction::Http(HttpAction { url: url.into() }))
}

pub fn http_fetch_new() {
    unsafe {
        super::raw::http_fetch();
    }
}
