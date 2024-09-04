use crate::{config::state::{AppState, AppStateDisplay}, util::response_util::ApiResponse};
use axum::{extract::State, response::IntoResponse};

pub async fn version() -> impl IntoResponse {
    ApiResponse::ok_data(env!("CARGO_PKG_VERSION"))
}

pub async fn state(State(app_state): State<AppState>) -> impl IntoResponse {
    ApiResponse::ok_data(AppStateDisplay {
        config: app_state.config.clone(),
        cycle: app_state.cycle.read().await.clone(),
    })
}
