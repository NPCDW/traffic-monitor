use std::any::Any;

use axum::http::Response;
use axum::response::IntoResponse;
use axum::{
    routing::get,
    Router,
};
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tower_http::services::ServeDir;
use tower_http::catch_panic::CatchPanicLayer;
use crate::controller::app_ctl;
use crate::config::state::AppState;
use crate::util::response_util::ApiResponse;

pub async fn init(app_state: AppState) -> Router {
    let app = Router::new()
        .route("/version", get(app_ctl::version))
        .route("/state", get(app_ctl::state));

    let api = Router::new()
        .nest("/app", app);

    let web = app_state.config.web.clone().unwrap();

    Router::new()
        .nest("/api", api)
        .layer(ValidateRequestHeaderLayer::bearer(&web.token))
        .nest_service("/", ServeDir::new(web.ui_path))
        .layer(CatchPanicLayer::custom(handle_panic))
        .with_state(app_state)
}

fn handle_panic(err: Box<dyn Any + Send + 'static>) -> Response<axum::body::Body> {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic message".to_string()
    };

    ApiResponse::<()>::error(&details).into_response()
}