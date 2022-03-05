#[macro_use]
extern crate log;
extern crate serde_json;
extern crate ulp;
// mod api;
// mod mft;
//
use ulp::workerpool::Orchestrator;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("ULP Starting - Initializing Orchestrator");
    let orchestrator = Orchestrator::default();
    let _ = orchestrator.run().await.unwrap();
}
