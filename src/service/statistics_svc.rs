use std::cmp::min;

use anyhow::anyhow;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use serde_json::json;

use crate::{config::state::{AppState, CycleAppState, CycleStatisticMethod, CycleType}, mapper::{monitor_day_mapper::{self, MonitorDay}, monitor_hour_mapper::{self, MonitorHour}, monitor_second_mapper::{self, MonitorSecond}}, service::systemstat_svc, util::http_util};

pub async fn frist_collect(app_state: &AppState) -> anyhow::Result<()> {
    generate_cycle(app_state).await?;
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

    let exceeds_limit = verify_exceeds_limit(app_state, (uplink_traffic_usage, downlink_traffic_usage)).await?;
    if exceeds_limit {
        tracing::warn!("流量超限");
    }

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

    if let Some(tg) = &app_state.config.tg {
        let mut text = format!("{}\n{} 用量\n上传: {}\n下载: {}", &app_state.config.vps_name, day.to_string(), traffic_show(uplink_traffic_usage), traffic_show(downlink_traffic_usage));
        let cycle = app_state.cycle.read().await.clone();
        if let Some(cycle) = cycle {
            if cycle.current_cycle_end_date < chrono::Local::now().date_naive() {
                return anyhow::Ok(());
            }
            let yesterday_traffic_usage = match cycle.statistic_method {
                CycleStatisticMethod::MaxInOut => std::cmp::max(uplink_traffic_usage, downlink_traffic_usage),
                CycleStatisticMethod::OnlyOut => uplink_traffic_usage,
                CycleStatisticMethod::SumInOut => uplink_traffic_usage + downlink_traffic_usage,
            };
            if cycle.current_cycle_start_date == chrono::Local::now().date_naive() {
                let pre_start = match cycle.cycle_type {
                    CycleType::DAY(each, _) =>  cycle.current_cycle_start_date - chrono::Duration::days(each),
                    CycleType::MONTH(each, _) => add_months(cycle.current_cycle_start_date, -each as i32),
                    _ => return Err(anyhow!("cycle_type 不会出现此类型")),
                };
                let pre_end = cycle.current_cycle_start_date - chrono::Duration::days(1);
                let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) = monitor_day_mapper::get_daterange_data(pre_start, pre_end, &app_state.db_pool).await?.unwrap_or((0, 0));
                let cycle_traffic_usage = match cycle.statistic_method {
                    CycleStatisticMethod::MaxInOut => std::cmp::max(cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage),
                    CycleStatisticMethod::OnlyOut => cycle_day_uplink_traffic_usage,
                    CycleStatisticMethod::SumInOut => cycle_day_uplink_traffic_usage + cycle_day_downlink_traffic_usage,
                };
                text = format!("\n计入流量: {}\n周期已用量:\n{}/{}\n上一周期已结束", traffic_show(yesterday_traffic_usage), traffic_show(cycle_traffic_usage + yesterday_traffic_usage), traffic_show(cycle.traffic_limit));
            } else {
                let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) = monitor_day_mapper::get_daterange_data(cycle.current_cycle_start_date, cycle.current_cycle_end_date, &app_state.db_pool).await?.unwrap_or((0, 0));
                let cycle_traffic_usage = match cycle.statistic_method {
                    CycleStatisticMethod::MaxInOut => std::cmp::max(cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage),
                    CycleStatisticMethod::OnlyOut => cycle_day_uplink_traffic_usage,
                    CycleStatisticMethod::SumInOut => cycle_day_uplink_traffic_usage + cycle_day_downlink_traffic_usage,
                };
                text = format!("\n计入流量: {}\n周期已用量:\n{}/{}\n当前周期: {} ~ {}\n距下次重置: {}天", traffic_show(yesterday_traffic_usage), traffic_show(cycle_traffic_usage + yesterday_traffic_usage), traffic_show(cycle.traffic_limit), cycle.current_cycle_start_date.to_string(), cycle.current_cycle_end_date.to_string(), (cycle.current_cycle_end_date - chrono::Local::now().date_naive()).num_days() + 1);
            }
        }
        let url = format!("https://api.telegram.org/bot{}/sendMessage", tg.bot_token);
        let body = json!({"chat_id": tg.chat_id, "text": text, "message_thread_id": tg.topic_id}).to_string();
        tracing::debug!("forward 消息 body: {}", &body);
        http_util::post(&url, body).await?;
    }

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

async fn verify_exceeds_limit(app_state: &AppState, (uplink_traffic_usage, downlink_traffic_usage): (i64, i64)) -> anyhow::Result<bool> {
    let config = &app_state.config;
    if config.liftcycle.is_none() {
        return anyhow::Ok(false);
    }
    let mut cycle = app_state.cycle.read().await.clone().unwrap();
    if cycle.current_cycle_end_date < chrono::Local::now().date_naive() {
        if let CycleType::ONCE(_, _) = cycle.cycle_type {
            return anyhow::Ok(false);
        }
        generate_cycle(app_state).await?;
        cycle = app_state.cycle.read().await.clone().unwrap();
    }
    let today_traffic_usage = match cycle.statistic_method {
        CycleStatisticMethod::MaxInOut => std::cmp::max(uplink_traffic_usage, downlink_traffic_usage),
        CycleStatisticMethod::OnlyOut => uplink_traffic_usage,
        CycleStatisticMethod::SumInOut => uplink_traffic_usage + downlink_traffic_usage,
    };
    let traffic_limit = cycle.traffic_limit;
    let traffic_usage = cycle.traffic_usage + today_traffic_usage;
    tracing::debug!("流量周期统计: 已用量: {} 限制: {}", traffic_show(traffic_usage), traffic_show(traffic_limit));
    cycle.traffic_usage = traffic_usage;
    *app_state.cycle.write().await = Some(cycle);
    if traffic_usage >= traffic_limit {
        return anyhow::Ok(true);
    }
    return anyhow::Ok(false);
}

async fn generate_cycle(app_state: &AppState) -> anyhow::Result<()> {
    let config = &app_state.config;
    if config.liftcycle.is_none() {
        return Err(anyhow!("config[liftcycle] 没有配置，生成流量周期失败"));
    }
    let liftcycle = config.liftcycle.as_ref().unwrap();
    let cycle_type = match liftcycle.cycle.as_str() {
        "day" => CycleType::DAY(liftcycle.each.unwrap(), chrono::NaiveDate::parse_from_str(liftcycle.traffic_reset_date.as_ref().unwrap(), "%Y-%m-%d")?),
        "month" => CycleType::MONTH(liftcycle.each.unwrap(), chrono::NaiveDate::parse_from_str(liftcycle.traffic_reset_date.as_ref().unwrap(), "%Y-%m-%d")?),
        "once" => CycleType::ONCE(chrono::NaiveDate::parse_from_str(liftcycle.start_date.as_ref().unwrap(), "%Y-%m-%d")?, chrono::NaiveDate::parse_from_str(liftcycle.end_date.as_ref().unwrap(), "%Y-%m-%d")?),
        _ => return Err(anyhow!("config[liftcycle][cycle] 配置填写错误，没有这样的类型")),
    };
    let now = chrono::Local::now().date_naive();
    let (current_cycle_start_date, current_cycle_end_date);
    if let CycleType::ONCE(start, end) = cycle_type {
        current_cycle_start_date = start;
        current_cycle_end_date = end;
    } else {
        let (each, mut traffic_reset_date) = match cycle_type {
            CycleType::DAY(each, traffic_reset_date) => (each, traffic_reset_date),
            CycleType::MONTH(each, traffic_reset_date) => (each, traffic_reset_date),
            _ => return Err(anyhow!("cycle_type 不会出现此类型")),
        };
        if each <= 0 {
            return Err(anyhow!("config[liftcycle][each] 配置填写错误，每多少天或每多少个月重置流量，必须是一个大于0的数"));
        }
        let add_or_sub = if now >= traffic_reset_date { 1 } else { -1 };
        loop {
            let end = match cycle_type {
                CycleType::DAY(each, traffic_reset_date) =>  traffic_reset_date + chrono::Duration::days(each * add_or_sub),
                CycleType::MONTH(each, traffic_reset_date) => add_months(traffic_reset_date, (each * add_or_sub) as i32),
                _ => return Err(anyhow!("cycle_type 不会出现此类型")),
            } - chrono::Duration::days(1);
            if now >= traffic_reset_date && now <= end {
                current_cycle_start_date = std::cmp::min(traffic_reset_date, end);
                current_cycle_end_date = std::cmp::max(traffic_reset_date, end);
                break;
            }
            traffic_reset_date = end;
        }
    }
    let statistic_method = match liftcycle.statistic_method.as_str() {
        "sum(in,out)" => CycleStatisticMethod::SumInOut,
        "max(in,out)" => CycleStatisticMethod::MaxInOut,
        "out" => CycleStatisticMethod::OnlyOut,
        _ => return Err(anyhow!("config[liftcycle][statistic_method] 配置填写错误，没有这样的类型")),
    };
    let traffic_limit = &liftcycle.traffic_limit.replace(" ", "").replace(",", "").replace("_", "");
    let traffic_limit = if let Some(traffic_limit) = traffic_limit.strip_suffix("MB") {
        let limit = traffic_limit.parse::<i64>()?;
        limit * 1024 * 1024
    } else if let Some(traffic_limit) = traffic_limit.strip_suffix("GB") {
        let limit = traffic_limit.parse::<i64>()?;
        limit * 1024 * 1024 * 1024
    } else if let Some(traffic_limit) = traffic_limit.strip_suffix("TB") {
        let limit = traffic_limit.parse::<i64>()?;
        limit * 1024 * 1024 * 1024 * 1024
    } else {
        return Err(anyhow!("config[liftcycle][traffic_limit] 需要以 MB GB TB 结尾"));
    };
    let now = chrono::Local::now().date_naive();
    let day_start_time = now.and_hms_opt(0, 0, 0).unwrap();
    let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) = monitor_day_mapper::get_daterange_data(current_cycle_start_date, current_cycle_end_date, &app_state.db_pool).await?.unwrap_or((0, 0));
    let (today_uplink_traffic_usage, today_downlink_traffic_usage) = monitor_second_mapper::get_timerange_data(day_start_time, day_start_time + Duration::days(1), &app_state.db_pool).await?.unwrap_or((0, 0));
    let traffic_usage = match statistic_method {
        CycleStatisticMethod::MaxInOut => std::cmp::max(cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) + std::cmp::max(today_uplink_traffic_usage, today_downlink_traffic_usage),
        CycleStatisticMethod::OnlyOut => cycle_day_uplink_traffic_usage + today_uplink_traffic_usage,
        CycleStatisticMethod::SumInOut => cycle_day_uplink_traffic_usage + cycle_day_downlink_traffic_usage + today_uplink_traffic_usage + today_downlink_traffic_usage,
    };
    *app_state.cycle.write().await = Some(CycleAppState {
        cycle_type,
        current_cycle_start_date,
        current_cycle_end_date,
        traffic_usage,
        traffic_limit,
        statistic_method,
    });
    anyhow::Ok(())
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let mut year = date.year();
    let month = date.month() as i32;
    let day = date.day();

    let mut new_month = month + months;
    if new_month > 12 {
        year += new_month / 12;
        new_month %= 12;
    } else if new_month < 1 {
        year += (new_month - 1) / 12;
        new_month = (new_month - 1) % 12 + 12;
    }

    let new_month = new_month as u32;
    let last_day_of_new_month = NaiveDate::from_ymd_opt(year, new_month, 1).unwrap()
        .with_day0(0).unwrap().day();
    let new_day = min(day, last_day_of_new_month);

    NaiveDate::from_ymd_opt(year, new_month, new_day).unwrap()
}