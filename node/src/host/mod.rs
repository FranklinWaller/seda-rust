mod batch_tick;
pub use batch_tick::*;

mod db_get;
pub use db_get::*;

mod db_set;
pub use db_set::*;

mod http_fetch;
pub use http_fetch::HttpFetch;
use rusqlite::params;
use seda_runtime::HostAdapter;
use tokio_rusqlite::Connection;

mod chain_call;
pub use chain_call::ChainCall;

mod chain_view;
pub use chain_view::ChainView;

mod trigger_event;
pub use trigger_event::TriggerEvent;

mod runtime_host;
use actix::prelude::*;
use futures::executor;
pub use runtime_host::*;

mod set_app_addr;
pub use set_app_addr::*;

use crate::{app::App, NodeError};

pub struct Host<HA: HostAdapter> {
    db_conn:        Connection,
    app_actor_addr: Option<Addr<App<HA>>>,
}

impl<HA: HostAdapter> Default for Host<HA> {
    fn default() -> Self {
        executor::block_on(async move {
            let db_conn = Connection::open("./seda_db.db3").await.expect("Couldn't open db conn");

            db_conn
                .call(|db_conn| {
                    db_conn
                        .execute(
                            "CREATE TABLE IF NOT EXISTS data (
                                key TEXT,
                                value TEXT NOT NULL
                            )",
                            params![],
                        )
                        .expect("couldn't create db table");

                    Ok::<_, NodeError>(())
                })
                .await
                .expect("Couldn't execute db call");

            Host {
                db_conn,
                app_actor_addr: None,
            }
        })
    }
}

impl<HA: HostAdapter> Actor for Host<HA> {
    type Context = Context<Self>;
}

impl<HA: HostAdapter> actix::Supervised for Host<HA> {}

impl<HA: HostAdapter> SystemService for Host<HA> {}
