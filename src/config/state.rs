use std::sync::Arc;

use sqlx::{Pool, Sqlite};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub enum CycleStatisticMethod {
    SumInOut,
    MaxInOut,
    OnlyOut,
}

#[derive(Clone, Debug)]
pub enum CycleType {
    DAY(i64, chrono::NaiveDate),
    MONTH(i64, chrono::NaiveDate),
    ONCE(chrono::NaiveDate, chrono::NaiveDate),
}

#[derive(Clone, Debug)]
pub struct CycleAppState {
    pub cycle_type: CycleType,
    pub current_cycle_start_date: chrono::NaiveDate,
    pub current_cycle_end_date: chrono::NaiveDate,
    pub traffic_usage: i64,
    pub traffic_limit: i64,
    pub statistic_method: CycleStatisticMethod,
}

#[derive(Clone)]
pub struct AppState {
    pub config: crate::config::app_config::Config,
    pub db_pool: Pool<Sqlite>,

    pub cycle: Arc<RwLock<Option<CycleAppState>>>,
}