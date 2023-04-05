use actix::prelude::*;
use serde::{Deserialize, Serialize};

use super::Host;

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "String")]
pub struct HttpFetch {
    pub url: String,
}

impl Handler<HttpFetch> for Host {
    type Result = ResponseActFuture<Self, String>;

    fn handle(&mut self, msg: HttpFetch, _ctx: &mut Self::Context) -> Self::Result {
        let fut = async { reqwest::get(msg.url).await.unwrap().text().await.unwrap() };

        Box::pin(fut.into_actor(self))
    }
}
