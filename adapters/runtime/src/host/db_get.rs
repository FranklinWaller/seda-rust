use actix::prelude::*;
use seda_chain_adapters::MainChainAdapterTrait;
use serde::{Deserialize, Serialize};

use crate::{Host, Result, RuntimeAdapterError};

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<Option<String>>")]
pub struct DatabaseGet {
    pub key: String,
}

impl<MC: MainChainAdapterTrait>  Handler<DatabaseGet> for Host<MC> {
    type Result = ResponseActFuture<Self, Result<Option<String>>>;

    fn handle(&mut self, msg: DatabaseGet, _ctx: &mut Self::Context) -> Self::Result {
        let db_conn = self.db_conn.clone();

        let fut = async move {
            let value = db_conn
                .call(move |conn| {
                    let mut stmt = conn.prepare("SELECT value FROM data WHERE key = ?1")?;
                    let mut retrieved: Option<String> = None;

                    stmt.query_row([msg.key], |row| {
                        retrieved = row.get(0)?;
                        Ok(())
                    })?;
                    Ok::<_, RuntimeAdapterError>(retrieved)
                })
                .await?;

            Ok(value)
        };

        Box::pin(fut.into_actor(self))
    }
}
