use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use sqlx::{Execute, Pool, QueryBuilder, Sqlite};

const ALL_FIELDS: &str = "id, create_time, day, hour, uplink_traffic_usage, downlink_traffic_usage";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default, sqlx::FromRow)]
pub struct MonitorHour {
    pub id: Option<u32>,
    pub create_time: Option<NaiveDateTime>,
    pub day: Option<NaiveDate>,
    pub hour: Option<u32>,
    pub uplink_traffic_usage: Option<i64>,
    pub downlink_traffic_usage: Option<i64>,
}

pub async fn update(
    entity: MonitorHour,
    pool: &Pool<Sqlite>,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("update monitor_hour set ");
    let mut separated = query_builder.separated(", ");
    if entity.day.is_some() {
        separated.push("day = ").push_bind_unseparated(entity.day.unwrap());
    }
    if entity.hour.is_some() {
        separated.push("hour = ").push_bind_unseparated(entity.hour.unwrap());
    }
    if entity.uplink_traffic_usage.is_some() {
        separated.push("uplink_traffic_usage = ").push_bind_unseparated(entity.uplink_traffic_usage.unwrap());
    }
    if entity.downlink_traffic_usage.is_some() {
        separated.push("downlink_traffic_usage = ").push_bind_unseparated(entity.downlink_traffic_usage.unwrap());
    }
    query_builder.push(" where id = ").push_bind(entity.id.unwrap());

    let query = query_builder.build();
    tracing::debug!("更新小时监控数据SQL: {}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("更新小时监控数据结果: {:?}", res);
    res
}

pub async fn create(
    entity: MonitorHour,
    pool: &Pool<Sqlite>,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
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
        separated.push_bind(entity.day.unwrap());
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
    tracing::debug!("插入小时监控数据SQL: {}", query.sql());
    let res = query.execute(pool).await;
    tracing::debug!("插入小时监控数据结果: {:?}", res);
    res
}

pub async fn sum_day_data(
    day: NaiveDate,
    pool: &Pool<Sqlite>,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        "select sum(uplink_traffic_usage), sum(downlink_traffic_usage) from monitor_hour where ",
    );
    query_builder.push("day = ").push_bind(day);
    let query = query_builder.build_query_as::<(i64, i64)>();
    tracing::debug!("查询一天的小时监控数据SQL: {}", query.sql());
    let res = query.fetch_optional(pool).await;
    tracing::debug!("查询一天的小时监控数据结果: {:?}", res);
    res
}

pub async fn get_day_hour_data(
    day: NaiveDate,
    hour: u32,
    pool: &Pool<Sqlite>,
) -> Result<Option<MonitorHour>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        format!("select {} from monitor_hour where ", ALL_FIELDS),
    );
    query_builder.push("day = ").push_bind(day).push(" and hour = ").push_bind(hour);
    let query = query_builder.build_query_as::<MonitorHour>();
    tracing::debug!("查询一天的小时监控数据SQL: {}", query.sql());
    let res = query.fetch_optional(pool).await;
    tracing::debug!("查询一天的小时监控数据结果: {:?}", res);
    res
}

pub async fn list_timerange_data(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    pool: &Pool<Sqlite>,
) -> Result<Vec<MonitorHour>, sqlx::Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new(
        format!("select {} from monitor_hour where ", ALL_FIELDS),
    );
    query_builder.push("(day = ").push_bind(start_time.date()).push(" and hour >= ").push_bind(start_time.hour()).push(")");
    query_builder.push("(day > ").push_bind(start_time.date()).push(" and day < ").push_bind(end_time.date()).push(")");
    query_builder.push("(day = ").push_bind(end_time.date()).push(" and hour < ").push_bind(end_time.hour()).push(")");
    let query = query_builder.build_query_as::<MonitorHour>();
    tracing::debug!("查询一天的小时监控数据SQL: {}", query.sql());
    let res = query.fetch_all(pool).await;
    tracing::debug!("查询一天的小时监控数据结果: {:?}", res);
    res
}
