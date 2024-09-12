use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CycleStatisticMethod {
    SumInOut,
    MaxInOut,
    OnlyOut,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CycleType {
    DAY(i64, chrono::NaiveDate),
    MONTH(i64, chrono::NaiveDate),
    ONCE(chrono::NaiveDate, chrono::NaiveDate),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CycleAppState {
    pub cycle_type: CycleType,
    pub current_cycle_start_date: chrono::NaiveDate,
    pub current_cycle_end_date: chrono::NaiveDate,
    pub uplink_traffic_usage: i64,
    pub downlink_traffic_usage: i64,
    pub traffic_usage: i64,
    pub traffic_limit: i64,
    pub notify_exceeds: bool,
    pub notify_half: bool,
    pub notify_80: bool,
    pub notify_90: bool,
    pub statistic_method: CycleStatisticMethod,
}

#[derive(Clone)]
pub struct AppState {
    pub config: crate::config::app_config::Config,
    pub db_pool: Pool<Sqlite>,

    pub cycle: Arc<RwLock<Option<CycleAppState>>>,
}

#[derive(Serialize, Deserialize)]
pub struct AppStateDisplay {
    pub config: crate::config::app_config::Config,
    pub cycle: Option<CycleAppState>,
}
