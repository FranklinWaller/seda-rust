use std::time::Duration;

use actix::{AsyncContext, Handler, Message};

use crate::{app::App, runtime_job::RuntimeJob};

/// The Job Manager’s job is to take events coming from P2P, tickers, RPC, etc
/// and give the task to the runtime when there is an available thread. Each
/// event comes with a ID that corresponds with that task. It’s important that
/// tasks that cannot be processed at the same time must have the same ID to
/// prevent this. For example with aggregating. (If you open two threads that
/// both do `counter += 1` the result would be `counter = 1` instead of `counter
/// = 2` and information will get lost)
///
/// The Job Manager is essentially pretty dumb, it takes the event and checks if
/// there is no thread currently running with that ID. If not and there is a
/// thread available, it spins up a new thread and gives the event information
/// along with some arguments.
#[derive(Message)]
#[rtype(result = "()")]
pub struct StartJobManager;

impl StartJobManager {
    const JOB_MANAGER_INTERVAL: u64 = 200;
}

impl Handler<StartJobManager> for App {
    type Result = ();

    fn handle(&mut self, msg: StartJobManager, ctx: &mut Self::Context) -> Self::Result {
        let mut event_queue = self.event_queue.write();
        let running_event_ids = self.running_event_ids.read();
        if let Some(event) = event_queue.get_next(running_event_ids.as_slice()) {
            self.runtime_worker.do_send(RuntimeJob { event });
        }

        ctx.notify_later(msg, Duration::from_millis(StartJobManager::JOB_MANAGER_INTERVAL));
    }
}
