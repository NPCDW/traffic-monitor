use anyhow::Ok;
use chrono::Timelike;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{config::state::AppState, mapper::monitor_second_mapper::{self, MonitorSecond}, mapper::monitor_hour_mapper::{self, MonitorHour}, mapper::monitor_day_mapper::{self, MonitorDay}};

use super::systemstat_svc;

pub async fn init(app_state: &AppState) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;
    
    let app_state_clone = app_state.clone();
    tracing::info!("开始收集监控数据");
    sched.add(Job::new_cron_job_async("0/15 * * * * ? ", move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let res = collect_second_data(&app_state).await;
            if res.is_err() {
                tracing::error!("收集秒级监控数据出错：{:?}", &res);
            }
        })
    })?).await?;

    let app_state_clone = app_state.clone();
    sched.add(Job::new_cron_job_async("0 1 * * * ? ", move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let res = collect_hour_data(&app_state).await;
            if res.is_err() {
                tracing::error!("收集小时监控数据出错：{:?}", &res);
            }
        })
    })?).await?;

    let app_state_clone = app_state.clone();
    sched.add(Job::new_cron_job_async("0 2 0 * * ? ", move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let res = collect_day_data(&app_state).await;
            if res.is_err() {
                tracing::error!("收集天监控数据出错：{:?}", &res);
            }
        })
    })?).await?;

    sched.start().await?;
    Ok(())
}

pub async fn collect_second_data(app_state: &AppState) -> anyhow::Result<()> {
    let traffic = systemstat_svc::traffic(&app_state.config.network_name)?;
    let (uplink_traffic_readings, downlink_traffic_readings) = (traffic.tx_bytes.0 as i64, traffic.rx_bytes.0 as i64);

    let pre_data = monitor_second_mapper::get_pre_data(&app_state.db_pool).await?;

    let end_time = chrono::Local::now();
    let (start_time, time_interval, uplink_traffic_usage, downlink_traffic_usage);
    if let Some(pre_data) = pre_data {
        start_time = pre_data.end_time.unwrap();
        time_interval = end_time.timestamp() - start_time.timestamp();
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

pub async fn collect_hour_data(app_state: &AppState) -> anyhow::Result<()> {
    let now = chrono::Local::now();
    let end_time = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let start_time = end_time - chrono::Duration::hours(1);
    let day = start_time.with_hour(0).unwrap();
    let res = monitor_second_mapper::get_timerange_data(start_time, end_time, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    let monitor_hour = MonitorHour {
        id: None,
        create_time: None,
        day: Some(day),
        hour: Some(start_time.hour()),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    monitor_hour_mapper::create(monitor_hour, &app_state.db_pool).await?;
    anyhow::Ok(())
}

pub async fn collect_day_data(app_state: &AppState) -> anyhow::Result<()> {
    let now = chrono::Local::now();
    let day = now.with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap() - chrono::Duration::days(1);
    let res = monitor_hour_mapper::get_day_data(day, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    let monitor_day = MonitorDay {
        id: None,
        create_time: None,
        day: Some(day),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    monitor_day_mapper::create(monitor_day, &app_state.db_pool).await?;
    anyhow::Ok(())
}
