use config::state::AppState;
// use service::systemstat_svc;

mod config;
mod mapper;
mod service;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::app_config::get_config();

    config::log::init(&config);
    tracing::debug!("Read Config: {:#?}", &config);

    service::signal_svc::handle();

    let db_pool = config::db::init().await?;

    let app_state = AppState {
        config: config,
        db_pool: db_pool,
    };

    let _ = service::scheduler_svc::init(&app_state).await;

    // systemstat_svc::test();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // anyhow::Ok(())
}
