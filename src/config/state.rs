use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub enum CycleStatisticMethod {
    SUM_IN_OUT,
    MAX_IN_OUT,
    ONLY_OUT,
}

#[derive(Clone)]
pub enum CycleType {
    DAY(u32, chrono::NaiveDate),
    MONTH(u32, chrono::NaiveDate),
    ONCE(chrono::NaiveDate, chrono::NaiveDate),
}

#[derive(Clone)]
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

    pub cycle: Option<CycleAppState>,
}