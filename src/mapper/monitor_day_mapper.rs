use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sqlx::{Execute, Pool, QueryBuilder, Sqlite};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, sqlx::FromRow)]
pub struct MonitorDay {
    pub id: Option<u32>,
    pub create_time: Option<DateTime<Local>>,
    pub day: Option<DateTime<Local>>,
    pub uplink_traffic_usage: Option<i64>,
    pub downlink_traffic_usage: Option<i64>,
}

pub async fn create(entity: MonitorDay, pool: &Pool<Sqlite>) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("insert into monitor_day(");
    let mut separated = query_builder.separated(", ");
    if entity.day.is_some() {
        separated.push("day");
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push("uplink_traffic_usage");
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push("downlink_traffic_usage");
    }
    query_builder.push(")  values(");
    let mut separated = query_builder.separated(", ");
    if entity.day.is_some() {
        separated.push("date(").push_bind_unseparated(entity.day.unwrap()).push_unseparated(")");
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push_bind(entity.uplink_traffic_usage.unwrap());
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push_bind(entity.downlink_traffic_usage.unwrap());
    }
    query_builder.push(")");

    let query = query_builder.build();
    tracing::debug!("插入天监控数据SQL：{}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("插入天监控数据结果：{:?}", res);
    res
}
