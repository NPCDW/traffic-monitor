use crate::{
    config::state::AppState, mapper::{monitor_day_mapper, monitor_hour_mapper, monitor_second_mapper::{self, MonitorSecond}}, service::statistics_svc, util::response_util::ApiResponse
};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModifyDataParam {
    pub uplink_traffic_usage: i64,
    pub downlink_traffic_usage: i64,
}

pub async fn modify_data(
    State(app_state): State<AppState>,
    body: Json<ModifyDataParam>,
) -> impl IntoResponse {
    let now = chrono::Local::now().naive_local();
    let data = MonitorSecond {
        id: None,
        create_time: None,
        start_time: Some(now),
        end_time: Some(now),
        uplink_traffic_readings: Some(0),
        downlink_traffic_readings: Some(0),
        uplink_traffic_usage: Some(body.uplink_traffic_usage),
        downlink_traffic_usage: Some(body.downlink_traffic_usage),
        time_interval: Some(0),
        is_corrected: Some(1),
    };
    match monitor_second_mapper::create(data, &app_state.db_pool).await {
        Ok(res) => {
            let _ = statistics_svc::collect_hour_data(&app_state, now).await;
        
            let _ = statistics_svc::collect_day_data(&app_state, now.date()).await;

            let _ = statistics_svc::verify_exceeds_limit(&app_state, (body.uplink_traffic_usage, body.downlink_traffic_usage)).await;
        
            return ApiResponse::ok_data(res.rows_affected())
        },
        Err(e) => return ApiResponse::error(&format!("添加数据失败: {}", e)),
    }
}

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
    start_time: String,
    end_time: String,
}

pub async fn list_monitor_hour(
    State(app_state): State<AppState>,
    body: Json<PageMonitorHourParam>,
) -> impl IntoResponse {
    let start_time = match NaiveDateTime::parse_from_str(&body.start_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("开始时间格式错误"),
    };
    let end_time = match NaiveDateTime::parse_from_str(&body.end_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("结束时间格式错误"),
    };
    match monitor_hour_mapper::list_timerange_data(start_time, end_time, &app_state.db_pool).await {
        Ok(list) => return ApiResponse::ok_data(list),
        Err(e) => return ApiResponse::error(&format!("查询数据失败: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageMonitorSecondParam {
    start_time: String,
    end_time: String,
}

pub async fn list_monitor_second(
    State(app_state): State<AppState>,
    body: Json<PageMonitorSecondParam>,
) -> impl IntoResponse {
    let start_time = match NaiveDateTime::parse_from_str(&body.start_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("开始时间格式错误"),
    };
    let end_time = match NaiveDateTime::parse_from_str(&body.end_time, "%Y-%m-%dT%H:%M:%S") {
        Ok(datetime) => datetime,
        Err(_) => return ApiResponse::error("结束时间格式错误"),
    };
    match monitor_second_mapper::list_timerange_data(start_time, end_time, &app_state.db_pool).await
    {
        Ok(list) => return ApiResponse::ok_data(list),
        Err(e) => return ApiResponse::error(&format!("查询数据失败: {}", e)),
    }
}

pub async fn send_today_statistics(
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    if app_state.config.tg.is_none() {
        return ApiResponse::error("未配置 TG bot");
    }
    match statistics_svc::tg_notify_daily_statistics(&app_state, chrono::Local::now().date_naive()).await {
        Ok(()) => return ApiResponse::ok_data(()),
        Err(e) => return ApiResponse::error(&format!("发送消息失败: {}", e)),
    }
}
