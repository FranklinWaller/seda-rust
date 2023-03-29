use clap::Subcommand;

mod batch;
mod bridge;
mod hello;
mod p2p;

#[derive(Debug, Subcommand)]
pub enum Task {
    Batch(batch::Batch),
    Bridge(bridge::Bridge),
    P2P(p2p::P2p),
    Hello(hello::Hello),
}

impl Task {
    pub fn handle(self) {
        match self {
            Self::Batch(bridge) => bridge.handle(),
            Self::Bridge(bridge) => bridge.handle(),
            Self::P2P(p2p) => p2p.handle(),
            Self::Hello(hello) => hello.handle(),
        }
    }
}
