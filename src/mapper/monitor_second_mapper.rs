use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use sqlx::{Execute, Pool, QueryBuilder, Sqlite};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, sqlx::FromRow)]
pub struct MonitorSecond {
    pub id: Option<u32>,
    pub create_time: Option<DateTime<Local>>,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
    pub uplink_traffic_readings: Option<i64>,
    pub downlink_traffic_readings: Option<i64>,
    pub uplink_traffic_usage: Option<i64>,
    pub downlink_traffic_usage: Option<i64>,
    pub time_interval: Option<i64>,
    pub is_corrected: Option<u32>,
}

pub async fn create(entity: MonitorSecond, pool: &Pool<Sqlite>) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("insert into monitor_second(");
    let mut separated = query_builder.separated(", ");
    if entity.start_time.is_some() {
        separated.push("start_time");
    }
    if entity.end_time.is_some() {
        separated.push("end_time");
    }
    if entity.uplink_traffic_readings.is_some() {
        separated.push("uplink_traffic_readings");
    }
    if entity.downlink_traffic_readings.is_some() {
        separated.push("downlink_traffic_readings");
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push("uplink_traffic_usage");
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push("downlink_traffic_usage");
    }
    if entity.time_interval.is_some() {
        separated.push("time_interval");
    }
    if entity.is_corrected.is_some() {
        separated.push("is_corrected");
    }
    query_builder.push(")  values(");
    let mut separated = query_builder.separated(", ");
    if entity.start_time.is_some() {
        separated.push("datetime(").push_bind_unseparated(entity.start_time.unwrap()).push_unseparated(",'localtime')");
    }
    if entity.end_time.is_some() {
        separated.push("datetime(").push_bind_unseparated(entity.end_time.unwrap()).push_unseparated(",'localtime')");
    }
    if entity.uplink_traffic_readings.is_some() {
        separated.push_bind(entity.uplink_traffic_readings.unwrap());
    }
    if entity.downlink_traffic_readings.is_some() {
        separated.push_bind(entity.downlink_traffic_readings.unwrap());
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push_bind(entity.uplink_traffic_usage.unwrap());
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push_bind(entity.downlink_traffic_usage.unwrap());
    }
    if entity.time_interval.is_some() {
        separated.push_bind(entity.time_interval.unwrap());
    }
    if entity.is_corrected.is_some() {
        separated.push_bind(entity.is_corrected.unwrap());
    }
    query_builder.push(")");

    let query = query_builder.build();
    tracing::debug!("插入秒级监控数据SQL：{}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("插入秒级监控数据结果：{:?}", res);
    res
}

pub async fn get_pre_data(pool: &Pool<Sqlite>) -> Result<Option<MonitorSecond>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("select * from monitor_second where is_corrected = false order by end_time desc limit 1");
    let query = query_builder.build_query_as::<MonitorSecond>();
    tracing::debug!("查询上一节点秒级监控数据SQL：{}", query.sql());
    let res = query.fetch_optional(pool).await;
    tracing::debug!("查询上一节点秒级监控数据结果：{:?}", res);
    res
}

pub async fn get_timerange_data(start_time: chrono::DateTime<Local>, end_time: chrono::DateTime<Local>, pool: &Pool<Sqlite>) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("select sum(uplink_traffic_usage), sum(downlink_traffic_usage) from monitor_second where ");
    query_builder.push("start_time >= datetime(").push_bind(start_time).push(",'localtime')");
    query_builder.push(" and start_time < datetime(").push_bind(end_time).push(",'localtime')");
    let query = query_builder.build_query_as::<(i64, i64)>();
    tracing::debug!("查询区域秒级监控数据SQL：{}", query.sql());
    let res = query.fetch_optional(pool).await;
    tracing::debug!("查询区域秒级监控数据结果：{:?}", res);
    res
}

pub async fn delete_by_date(date: chrono::DateTime<Local>, pool: &Pool<Sqlite>) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let mut query_builder = QueryBuilder::new("delete from monitor_second where ");
    query_builder.push("end_time < datetime(").push_bind(date).push(",'localtime')");
    let query = query_builder.build();
    tracing::debug!("删除秒级监控数据SQL：{}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("删除秒级监控数据结果：{:?}", res);
    res
}