use std::sync::Arc;

use config::state::AppState;
use tokio::sync::RwLock;
// use service::systemstat_svc;

mod config;
mod controller;
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
        cycle: Arc::new(RwLock::new(None)),
    };

    service::statistics_svc::frist_collect(&app_state).await?;

    service::scheduler_svc::init(&app_state).await?;

    // systemstat_svc::test();

    let app_state_clone = app_state.clone();
    if let Some(web) = app_state.config.web {
        let router = config::route::init(app_state_clone).await;
        let listener = tokio::net::TcpListener::bind(web.listener).await?;
        tracing::info!("listening on {:?}", listener);
        axum::serve(listener, router)
            .await
            .unwrap_or_else(|e| panic!("start service fail {:#?}", e));
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // anyhow::Ok(())
}
