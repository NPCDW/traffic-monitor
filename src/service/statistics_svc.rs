use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use serde_json::json;

use crate::{config::state::AppState, mapper::{monitor_day_mapper::{self, MonitorDay}, monitor_hour_mapper::{self, MonitorHour}, monitor_second_mapper::{self, MonitorSecond}}, service::systemstat_svc, util::http_util};

pub async fn frist_collect(app_state: &AppState) -> anyhow::Result<()> {
    let now = chrono::Local::now().naive_local();
    let pre_data = monitor_second_mapper::get_pre_data(&app_state.db_pool).await?;
    if let None = pre_data {
        return anyhow::Ok(());
    }
    let pre_data = pre_data.unwrap();
    let pre_end_time = pre_data.end_time.unwrap();
    if now - pre_end_time < chrono::Duration::seconds(15) {
        return anyhow::Ok(());
    }
    collect_second_data(app_state).await?;
    if now.date() == pre_end_time.date() && now.hour() == pre_end_time.hour() {
        return anyhow::Ok(());
    }
    collect_hour_data(app_state, pre_end_time).await?;
    if now.date() == pre_end_time.date() {
        return anyhow::Ok(());
    }
    collect_day_data(app_state, pre_end_time.date()).await?;
    anyhow::Ok(())
}

pub async fn collect_second_data(app_state: &AppState) -> anyhow::Result<()> {
    let traffic = systemstat_svc::traffic(&app_state.config.network_name)?;
    let (uplink_traffic_readings, downlink_traffic_readings) = (traffic.tx_bytes.0 as i64, traffic.rx_bytes.0 as i64);

    let pre_data = monitor_second_mapper::get_pre_data(&app_state.db_pool).await?;

    let end_time = chrono::Local::now().naive_local();
    let (start_time, time_interval, uplink_traffic_usage, downlink_traffic_usage);
    if let Some(pre_data) = pre_data {
        start_time = pre_data.end_time.unwrap();
        time_interval = (end_time - start_time).num_seconds();
        let pre_uplink_traffic_readings = pre_data.uplink_traffic_readings.unwrap();
        let pre_downlink_traffic_readings = pre_data.downlink_traffic_readings.unwrap();
        uplink_traffic_usage = if pre_uplink_traffic_readings > uplink_traffic_readings { uplink_traffic_readings } else { uplink_traffic_readings - pre_uplink_traffic_readings };
        downlink_traffic_usage = if pre_downlink_traffic_readings > downlink_traffic_readings { downlink_traffic_readings } else { downlink_traffic_readings - pre_downlink_traffic_readings };
    } else {
        start_time = end_time;
        uplink_traffic_usage = 0;
        downlink_traffic_usage = 0;
        time_interval = 0;
    }
    tracing::debug!("秒统计: {} ~ {} 上行: {} 下行: {}", &start_time.to_string(), end_time.to_string(), traffic_show(uplink_traffic_usage), traffic_show(downlink_traffic_usage));
    let monitor_second = MonitorSecond {
        id: None,
        create_time: None,
        start_time: Some(start_time),
        end_time: Some(end_time),
        uplink_traffic_readings: Some(uplink_traffic_readings),
        downlink_traffic_readings: Some(downlink_traffic_readings),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
        time_interval: Some(time_interval),
        is_corrected: Some(0),
    };
    monitor_second_mapper::create(monitor_second, &app_state.db_pool).await?;
    anyhow::Ok(())
}

pub async fn collect_hour_data(app_state: &AppState, statistic_hour_time: NaiveDateTime) -> anyhow::Result<()> {
    let start_time = statistic_hour_time.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let end_time = start_time + chrono::Duration::hours(1);
    let day = start_time.with_hour(0).unwrap();
    let res = monitor_second_mapper::get_timerange_data(start_time, end_time, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    tracing::info!("小时统计: {} {} 上行: {} 下行: {}", &day.date().to_string(), start_time.hour(), traffic_show(uplink_traffic_usage), traffic_show(downlink_traffic_usage));
    let monitor_hour = MonitorHour {
        id: None,
        create_time: None,
        day: Some(day.date()),
        hour: Some(start_time.hour()),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    monitor_hour_mapper::create(monitor_hour, &app_state.db_pool).await?;
    anyhow::Ok(())
}

pub async fn collect_day_data(app_state: &AppState, statistic_date: NaiveDate) -> anyhow::Result<()> {
    let res = monitor_hour_mapper::get_day_data(statistic_date, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    tracing::info!("天统计: {} 上行: {} 下行: {}", &statistic_date.to_string(), traffic_show(uplink_traffic_usage), traffic_show(downlink_traffic_usage));
    let monitor_day = MonitorDay {
        id: None,
        create_time: None,
        day: Some(statistic_date),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    monitor_day_mapper::create(monitor_day, &app_state.db_pool).await?;
    
    let day = statistic_date - chrono::Duration::days(1);
    monitor_second_mapper::delete_by_date(day.and_time(NaiveTime::from_hms_milli_opt(0, 0, 0, 0).unwrap()), &app_state.db_pool).await?;

    let text = format!("{} [{}]\n上传: {} 下载: {}", day.to_string(), &app_state.config.vps_name, traffic_show(uplink_traffic_usage), traffic_show(downlink_traffic_usage));
    let url = format!("https://api.telegram.org/bot{}/sendMessage", &app_state.config.tg.bot_token);
    let body = json!({"chat_id": &app_state.config.tg.chat_id, "text": text, "message_thread_id": &app_state.config.tg.topic_id}).to_string();
    tracing::debug!("forward 消息 body: {}", &body);
    http_util::post(&url, body).await?;

    anyhow::Ok(())
}

fn traffic_show(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes < KB {
        return format!("{} B", bytes);
    } else if bytes < MB {
        return format!("{:.2} KB", bytes as f64 / KB as f64);
    } else if bytes < GB {
        return format!("{:.2} MB", bytes as f64 / MB as f64);
    } else if bytes < TB {
        return format!("{:.2} GB", bytes as f64 / GB as f64);
    } else {
        return format!("{:.2} TB", bytes as f64 / TB as f64);
    }
}