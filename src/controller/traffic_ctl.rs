use crate::{
    config::state::AppState,
    mapper::{monitor_day_mapper, monitor_hour_mapper, monitor_second_mapper},
    util::response_util::ApiResponse,
};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageMonitorDayParam {
    start_date: String,
    end_date: String,
}

pub async fn list_monitor_day(
    State(app_state): State<AppState>,
    body: Json<PageMonitorDayParam>,
) -> impl IntoResponse {
    let start_date = match NaiveDate::parse_from_str(&body.start_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return ApiResponse::error("开始日期格式错误"),
    };
    let end_date = match NaiveDate::parse_from_str(&body.end_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return ApiResponse::error("结束日期格式错误"),
    };
    match monitor_day_mapper::list_daterange_data(start_date, end_date, &app_state.db_pool).await {
        Ok(list) => return ApiResponse::ok_data(list),
        Err(e) => return ApiResponse::error(&format!("查询数据失败: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageMonitorHourParam {
    day: String,
}

pub async fn list_monitor_hour(
    State(app_state): State<AppState>,
    body: Json<PageMonitorHourParam>,
) -> impl IntoResponse {
    let day = match NaiveDate::parse_from_str(&body.day, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return ApiResponse::error("开始日期格式错误"),
    };
    match monitor_hour_mapper::list_day_data(day, &app_state.db_pool).await {
        Ok(list) => return ApiResponse::ok_data(list),
        Err(e) => return ApiResponse::error(&format!("查询数据失败: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageMonitorSecondParam {
    start_time: String,
    end_time: String,
    page_number: u32,
    page_size: u32,
}

pub async fn list_monitor_second(
    State(app_state): State<AppState>,
    body: Json<PageMonitorSecondParam>,
) -> impl IntoResponse {
    let start_time = match NaiveDateTime::parse_from_str(&body.start_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("开始日期格式错误"),
    };
    let end_time = match NaiveDateTime::parse_from_str(&body.end_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("结束日期格式错误"),
    };
    match monitor_second_mapper::list_timerange_data(start_time, end_time, &app_state.db_pool).await
    {
        Ok(list) => return ApiResponse::ok_data(list),
        Err(e) => return ApiResponse::error(&format!("查询数据失败: {}", e)),
    }
}
