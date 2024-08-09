use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sqlx::{Execute, Pool, QueryBuilder, Sqlite};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, sqlx::FromRow)]
pub struct MonitorHour {
    pub id: Option<u32>,
    pub create_time: Option<DateTime<Local>>,
    pub day: Option<DateTime<Local>>,
    pub hour: Option<u32>,
    pub uplink_traffic_usage: Option<i64>,
    pub downlink_traffic_usage: Option<i64>,
}

pub async fn create(entity: MonitorHour, pool: &Pool<Sqlite>) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("insert into monitor_hour(");
    let mut separated = query_builder.separated(", ");
    if entity.day.is_some() {
        separated.push("day");
    }
    if entity.hour.is_some() {
        separated.push("hour");
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
    if entity.hour.is_some() {
        separated.push_bind(entity.hour.unwrap());
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push_bind(entity.uplink_traffic_usage.unwrap());
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push_bind(entity.downlink_traffic_usage.unwrap());
    }
    query_builder.push(")");

    let query = query_builder.build();
    tracing::debug!("插入小时监控数据SQL：{}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("插入小时监控数据结果：{:?}", res);
    res
}

pub async fn get_day_data(day: chrono::DateTime<Local>, pool: &Pool<Sqlite>) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("select sum(uplink_traffic_usage), sum(downlink_traffic_usage) from monitor_hour where ");
    query_builder.push("day = date(").push_bind(day).push(")");
    let query = query_builder.build_query_as::<(i64, i64)>();
    tracing::debug!("查询一天的小时监控数据SQL：{}", query.sql());
    let res = query.fetch_optional(pool).await;
    tracing::debug!("查询一天的小时监控数据结果：{:?}", res);
    res
}
