use service::systemstat_svc;

mod config;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    config::log::init();

    service::signal_svc::handle();

    systemstat_svc::test();

    anyhow::Ok(())
}
