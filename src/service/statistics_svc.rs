use std::str::FromStr;

use anyhow::anyhow;
use chrono::{Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use rust_decimal_macros::dec;
use serde_json::json;

use crate::{
    config::state::{AppState, CycleAppState, CycleNotifyAppState, CycleStatisticMethod, CycleType},
    mapper::{
        monitor_day_mapper::{self, MonitorDay},
        monitor_hour_mapper::{self, MonitorHour},
        monitor_second_mapper::{self, MonitorSecond},
    },
    service::systemstat_svc,
    util::{command_util, http_util, tg_util},
};

const KB: i64 = 1024;
const MB: i64 = KB * 1024;
const GB: i64 = MB * 1024;
const TB: i64 = GB * 1024;

pub async fn frist_collect(app_state: &AppState) -> anyhow::Result<()> {
    generate_cycle(app_state).await?;
    let now = chrono::Local::now().naive_local();
    let pre_data = monitor_second_mapper::get_pre_data(&app_state.db_pool).await?;
    if let None = pre_data {
        return anyhow::Ok(());
    }
    let pre_data = pre_data.unwrap();
    let pre_end_time = pre_data.end_time.unwrap();
    if now - pre_end_time < chrono::Duration::seconds(15) && now.minute() == pre_end_time.minute() {
        return anyhow::Ok(());
    }
    collect_second_data(app_state).await?;
    anyhow::Ok(())
}

pub async fn collect_second_data(app_state: &AppState) -> anyhow::Result<()> {
    let traffic = systemstat_svc::traffic(&app_state.config.network_name)?;
    let (uplink_traffic_readings, downlink_traffic_readings) =
        (traffic.tx_bytes.0 as i64, traffic.rx_bytes.0 as i64);

    let pre_data = monitor_second_mapper::get_pre_data(&app_state.db_pool).await?;

    let end_time = chrono::Local::now().naive_local();
    let (start_time, time_interval, uplink_traffic_usage, downlink_traffic_usage);
    if let Some(pre_data) = pre_data {
        start_time = pre_data.end_time.unwrap();
        time_interval = (end_time - start_time).num_seconds();
        let pre_uplink_traffic_readings = pre_data.uplink_traffic_readings.unwrap();
        let pre_downlink_traffic_readings = pre_data.downlink_traffic_readings.unwrap();
        uplink_traffic_usage = if pre_uplink_traffic_readings > uplink_traffic_readings {
            uplink_traffic_readings
        } else {
            uplink_traffic_readings - pre_uplink_traffic_readings
        };
        downlink_traffic_usage = if pre_downlink_traffic_readings > downlink_traffic_readings {
            downlink_traffic_readings
        } else {
            downlink_traffic_readings - pre_downlink_traffic_readings
        };
    } else {
        start_time = end_time;
        uplink_traffic_usage = 0;
        downlink_traffic_usage = 0;
        time_interval = 0;
    }
    tracing::debug!(
        "秒统计: {} ~ {} 上行: {} 下行: {}",
        &start_time.to_string(),
        end_time.to_string(),
        traffic_show(uplink_traffic_usage),
        traffic_show(downlink_traffic_usage)
    );
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

    collect_hour_data(app_state, start_time).await?;

    collect_day_data(app_state, start_time.date()).await?;

    verify_exceeds_limit(app_state, (uplink_traffic_usage, downlink_traffic_usage)).await?;

    anyhow::Ok(())
}

pub async fn collect_hour_data(
    app_state: &AppState,
    statistic_hour_time: NaiveDateTime,
) -> anyhow::Result<()> {
    let start_time = statistic_hour_time
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let end_time = start_time + chrono::Duration::hours(1);
    let day = start_time.with_hour(0).unwrap();
    let res =
        monitor_second_mapper::sum_timerange_data(start_time, end_time, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    tracing::debug!(
        "小时统计: {} {} 上行: {} 下行: {}",
        &day.date().to_string(),
        start_time.hour(),
        traffic_show(uplink_traffic_usage),
        traffic_show(downlink_traffic_usage)
    );
    let mut monitor_hour = MonitorHour {
        id: None,
        create_time: None,
        day: Some(day.date()),
        hour: Some(start_time.hour()),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    let entity = monitor_hour_mapper::get_day_hour_data(day.date(), start_time.hour(), &app_state.db_pool).await?;
    if let Some(entity) = entity {
        monitor_hour.id = entity.id;
        monitor_hour_mapper::update(monitor_hour, &app_state.db_pool).await?;
    } else {
        monitor_hour_mapper::create(monitor_hour, &app_state.db_pool).await?;
    }
    anyhow::Ok(())
}

pub async fn collect_day_data(
    app_state: &AppState,
    statistic_date: NaiveDate,
) -> anyhow::Result<()> {
    let res = monitor_hour_mapper::sum_day_data(statistic_date, &app_state.db_pool).await?;
    if res.is_none() {
        return anyhow::Ok(());
    }
    let (uplink_traffic_usage, downlink_traffic_usage) = res.unwrap();
    tracing::debug!(
        "天统计: {} 上行: {} 下行: {}",
        &statistic_date.to_string(),
        traffic_show(uplink_traffic_usage),
        traffic_show(downlink_traffic_usage)
    );
    let mut monitor_day = MonitorDay {
        id: None,
        create_time: None,
        day: Some(statistic_date),
        uplink_traffic_usage: Some(uplink_traffic_usage),
        downlink_traffic_usage: Some(downlink_traffic_usage),
    };
    let entity = monitor_day_mapper::get_day_data(statistic_date, &app_state.db_pool).await?;
    if let Some(entity) = entity {
        monitor_day.id = entity.id;
        monitor_day_mapper::update(monitor_day, &app_state.db_pool).await?;
    } else {
        monitor_day_mapper::create(monitor_day, &app_state.db_pool).await?;
    }

    let day = statistic_date - chrono::Duration::days(1);
    monitor_second_mapper::delete_by_date(
        day.and_time(NaiveTime::from_hms_milli_opt(0, 0, 0, 0).unwrap()),
        &app_state.db_pool,
    )
    .await?;

    anyhow::Ok(())
}

pub async fn tg_notify_daily_statistics(app_state: &AppState, day: NaiveDate) -> anyhow::Result<()> {
    let tg = match &app_state.config.tg {
        Some(tg) => tg,
        None => return anyhow::Ok(()),
    };
    if !tg.daily_report {
        return anyhow::Ok(());
    }
    let entity = match monitor_day_mapper::get_day_data(day, &app_state.db_pool).await? {
        Some(entity) => entity,
        None => return Err(anyhow!("未找到当天的统计数据")),
    };
    let uplink_traffic_usage = entity.uplink_traffic_usage.unwrap();
    let downlink_traffic_usage = entity.downlink_traffic_usage.unwrap();
    let mut text = format!(
        "{}\n{} 上传: {} 下载: {}",
        &app_state.config.vps_name,
        day.to_string(),
        traffic_show(uplink_traffic_usage),
        traffic_show(downlink_traffic_usage)
    );
    let cycle = app_state.cycle.read().await.clone();
    if let Some(cycle) = cycle {
        if cycle.current_cycle_end_date < chrono::Local::now().date_naive() {
            return anyhow::Ok(());
        }
        let yesterday_traffic_usage = match cycle.statistic_method {
            CycleStatisticMethod::MaxInOut => {
                std::cmp::max(uplink_traffic_usage, downlink_traffic_usage)
            }
            CycleStatisticMethod::OnlyOut => uplink_traffic_usage,
            CycleStatisticMethod::SumInOut => uplink_traffic_usage + downlink_traffic_usage,
        };
        if cycle.current_cycle_start_date == chrono::Local::now().date_naive() {
            let pre_start = match cycle.cycle_type {
                CycleType::DAY(each, _) => {
                    cycle.current_cycle_start_date - chrono::Duration::days(each)
                }
                CycleType::MONTH(each, _) => cycle
                    .current_cycle_start_date
                    .checked_sub_months(Months::new(each as u32))
                    .unwrap(),
                _ => return Err(anyhow!("cycle_type 不会出现此类型")),
            };
            let pre_end = cycle.current_cycle_start_date - chrono::Duration::days(1);
            let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) =
                monitor_day_mapper::sum_daterange_data(pre_start, pre_end, &app_state.db_pool)
                    .await?
                    .unwrap_or((0, 0));
            let cycle_traffic_usage = match cycle.statistic_method {
                CycleStatisticMethod::MaxInOut => std::cmp::max(
                    cycle_day_uplink_traffic_usage,
                    cycle_day_downlink_traffic_usage,
                ),
                CycleStatisticMethod::OnlyOut => cycle_day_uplink_traffic_usage,
                CycleStatisticMethod::SumInOut => {
                    cycle_day_uplink_traffic_usage + cycle_day_downlink_traffic_usage
                }
            };
            text = format!(
                "{} 计入流量: {}\n{} ~ {} 上传: {} 下载: {} 计入流量: {}/{}\n上一周期已结束\n剩余流量 {}%",
                text,
                traffic_show(yesterday_traffic_usage),
                pre_start,
                pre_end,
                traffic_show(cycle_day_uplink_traffic_usage),
                traffic_show(cycle_day_downlink_traffic_usage),
                traffic_show(cycle_traffic_usage),
                traffic_show(cycle.traffic_limit),
                format!("{:.0}", Decimal::from_i64(cycle.traffic_limit - cycle_traffic_usage).unwrap() / Decimal::from_i64(cycle.traffic_limit).unwrap() * Decimal::from_i64(100).unwrap())
            );
        } else {
            let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) =
                monitor_day_mapper::sum_daterange_data(
                    cycle.current_cycle_start_date,
                    chrono::Local::now().date_naive() - chrono::Duration::days(1),
                    &app_state.db_pool,
                )
                .await?
                .unwrap_or((0, 0));
            let cycle_traffic_usage = match cycle.statistic_method {
                CycleStatisticMethod::MaxInOut => std::cmp::max(
                    cycle_day_uplink_traffic_usage,
                    cycle_day_downlink_traffic_usage,
                ),
                CycleStatisticMethod::OnlyOut => cycle_day_uplink_traffic_usage,
                CycleStatisticMethod::SumInOut => {
                    cycle_day_uplink_traffic_usage + cycle_day_downlink_traffic_usage
                }
            };
            let remain_day = (cycle.current_cycle_end_date - chrono::Local::now().date_naive()).num_days() + 1;
            let total_day = (cycle.current_cycle_end_date - cycle.current_cycle_start_date).num_days() + 1;
            text = format!(
                "{} 计入流量: {}\n{} ~ {} 上传: {} 下载: {} 计入流量: {}/{}\n距下次重置: {}天\n剩余流量 {}% 剩余周期 {}%",
                text,
                traffic_show(yesterday_traffic_usage),
                cycle.current_cycle_start_date,
                cycle.current_cycle_end_date,
                traffic_show(cycle_day_uplink_traffic_usage),
                traffic_show(cycle_day_downlink_traffic_usage),
                traffic_show(cycle_traffic_usage),
                traffic_show(cycle.traffic_limit),
                remain_day,
                format!("{:.0}", Decimal::from_i64(cycle.traffic_limit - cycle_traffic_usage).unwrap() / Decimal::from_i64(cycle.traffic_limit).unwrap() * Decimal::from_i64(100).unwrap()),
                format!("{:.0}", Decimal::from_i64(remain_day).unwrap() / Decimal::from_i64(total_day).unwrap() * Decimal::from_i64(100).unwrap()),
            );
        }
    }
    let url = format!("https://api.telegram.org/bot{}/sendMessage", tg.bot_token);
    let body = json!({"chat_id": tg.chat_id, "text": text, "parse_mode": "Markdown", "message_thread_id": tg.topic_id}).to_string();
    tracing::debug!("每日报告消息 body: {}", &body);
    match http_util::post(&url, body).await {
        Ok(_) => tracing::info!("tg 每日报告消息发送成功"),
        Err(e) => tracing::error!("tg 消息发送失败: {}", e),
    }
    
    anyhow::Ok(())
}

fn traffic_show<T: Into<Decimal>>(bytes: T) -> String {
    let bytes = bytes.into();
    if bytes < Decimal::from_i64(KB).unwrap() {
        return format!("{} B", bytes);
    } else if bytes < Decimal::from_i64(MB).unwrap() {
        return format!("{:.2} KB", bytes / Decimal::from_i64(KB).unwrap());
    } else if bytes < Decimal::from_i64(GB).unwrap() {
        return format!("{:.2} MB", bytes / Decimal::from_i64(MB).unwrap());
    } else if bytes < Decimal::from_i64(TB).unwrap() {
        return format!("{:.2} GB", bytes / Decimal::from_i64(GB).unwrap());
    } else {
        return format!("{:.2} TB", bytes / Decimal::from_i64(TB).unwrap());
    }
}

pub async fn verify_exceeds_limit(
    app_state: &AppState,
    (uplink_traffic_usage, downlink_traffic_usage): (i64, i64),
) -> anyhow::Result<()> {
    let config = &app_state.config;
    if config.traffic_cycle.is_none() {
        return anyhow::Ok(());
    }
    let mut cycle = app_state.cycle.read().await.clone().unwrap();
    if cycle.current_cycle_end_date < chrono::Local::now().date_naive() {
        if let CycleType::ONCE(_, _) = cycle.cycle_type {
            return anyhow::Ok(());
        }
        generate_cycle(app_state).await?;
        cycle = app_state.cycle.read().await.clone().unwrap();
    }
    cycle.uplink_traffic_usage = cycle.uplink_traffic_usage + uplink_traffic_usage;
    cycle.downlink_traffic_usage = cycle.downlink_traffic_usage + downlink_traffic_usage;
    cycle.traffic_usage = match cycle.statistic_method {
        CycleStatisticMethod::MaxInOut => {
            std::cmp::max(cycle.uplink_traffic_usage, cycle.downlink_traffic_usage)
        }
        CycleStatisticMethod::OnlyOut => cycle.uplink_traffic_usage,
        CycleStatisticMethod::SumInOut => cycle.uplink_traffic_usage + cycle.downlink_traffic_usage,
    };
    let traffic_limit = Decimal::from_i64(cycle.traffic_limit).unwrap();
    let traffic_usage = Decimal::from_i64(cycle.traffic_usage).unwrap();
    tracing::debug!(
        "流量周期统计: 已用量: {} 限制: {}",
        traffic_show(traffic_usage),
        traffic_show(traffic_limit)
    );
    for notify in &mut cycle.notify {
        if traffic_usage >= traffic_limit / dec!(100) * Decimal::from_u8(notify.percent).unwrap() {
            if !notify.finished {
                tracing::warn!("{} 流量使用超{}%", config.vps_name, notify.percent);
                let text = format!(
                    "{} 流量使用超{}% {}/{}",
                    config.vps_name,
                    notify.percent,
                    traffic_show(traffic_usage),
                    traffic_show(traffic_limit)
                );
                tg_util::send_msg(config, text).await;
                if let Some(exec) = &notify.exec {
                    tracing::info!("流量使用超出限制，执行命令: {}", exec);
                    match command_util::execute_to_output(".".to_string(), vec![exec.clone()]).await {
                        Ok(res) => {
                            if res.status.success() {
                                tracing::info!(
                                    "执行命令成功，执行结果: {}",
                                    String::from_utf8_lossy(&res.stdout)
                                )
                            } else {
                                tracing::info!(
                                    "执行命令失败，执行结果: {}",
                                    String::from_utf8_lossy(&res.stderr)
                                )
                            }
                        }
                        Err(e) => tracing::info!("命令提交失败: {:?}", e),
                    }
                }
                notify.finished = true;
            }
            break;
        }
    }
    *app_state.cycle.write().await = Some(cycle);
    return anyhow::Ok(());
}

async fn generate_cycle(app_state: &AppState) -> anyhow::Result<()> {
    let config = &app_state.config;
    if config.traffic_cycle.is_none() {
        tracing::info!("config[traffic_cycle] 没有配置，不生成流量周期");
        return anyhow::Ok(());
    }
    let liftcycle = config.traffic_cycle.as_ref().unwrap();
    let cycle_type = match liftcycle.cycle_type.as_str() {
        "day" => CycleType::DAY(
            liftcycle.each.unwrap(),
            chrono::NaiveDate::parse_from_str(
                liftcycle.traffic_reset_date.as_ref().unwrap(),
                "%Y-%m-%d",
            )?,
        ),
        "month" => CycleType::MONTH(
            liftcycle.each.unwrap(),
            chrono::NaiveDate::parse_from_str(
                liftcycle.traffic_reset_date.as_ref().unwrap(),
                "%Y-%m-%d",
            )?,
        ),
        "once" => CycleType::ONCE(
            chrono::NaiveDate::parse_from_str(liftcycle.start_date.as_ref().unwrap(), "%Y-%m-%d")?,
            chrono::NaiveDate::parse_from_str(liftcycle.end_date.as_ref().unwrap(), "%Y-%m-%d")?,
        ),
        _ => {
            return Err(anyhow!(
                "config[liftcycle][cycle] 配置填写错误，没有这样的类型"
            ))
        }
    };
    let now = chrono::Local::now().date_naive();
    let (mut current_cycle_start_date, mut current_cycle_end_date);
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
                CycleType::DAY(each, _) => {
                    traffic_reset_date + chrono::Duration::days(each * add_or_sub)
                }
                CycleType::MONTH(each, _) => {
                    if add_or_sub == 1 {
                        traffic_reset_date
                            .checked_add_months(Months::new(each as u32))
                            .unwrap()
                    } else {
                        traffic_reset_date
                            .checked_sub_months(Months::new(each as u32))
                            .unwrap()
                    }
                }
                _ => return Err(anyhow!("cycle_type 不会出现此类型")),
            };
            current_cycle_start_date = std::cmp::min(traffic_reset_date, end);
            current_cycle_end_date =
                std::cmp::max(traffic_reset_date, end) - chrono::Duration::days(1);
            if now >= current_cycle_start_date && now <= current_cycle_end_date {
                break;
            }
            traffic_reset_date = end;
        }
    }
    let statistic_method = match liftcycle.statistic_method.as_str() {
        "sum(in,out)" => CycleStatisticMethod::SumInOut,
        "max(in,out)" => CycleStatisticMethod::MaxInOut,
        "out" => CycleStatisticMethod::OnlyOut,
        _ => {
            return Err(anyhow!(
                "config[liftcycle][statistic_method] 配置填写错误，没有这样的类型"
            ))
        }
    };
    let traffic_limit = &liftcycle
        .traffic_limit
        .replace(" ", "")
        .replace(",", "")
        .replace("_", "");
    let traffic_limit = if let Some(traffic_limit) = traffic_limit.strip_suffix("MB") {
        Decimal::from_str(traffic_limit).unwrap() * Decimal::from_i64(MB).unwrap()
    } else if let Some(traffic_limit) = traffic_limit.strip_suffix("GB") {
        Decimal::from_str(traffic_limit).unwrap() * Decimal::from_i64(GB).unwrap()
    } else if let Some(traffic_limit) = traffic_limit.strip_suffix("TB") {
        Decimal::from_str(traffic_limit).unwrap() * Decimal::from_i64(TB).unwrap()
    } else {
        return Err(anyhow!(
            "config[liftcycle][traffic_limit] 需要以 MB GB TB 结尾"
        ));
    };
    let traffic_limit = traffic_limit.to_string().parse::<i64>()?;
    let now = chrono::Local::now().date_naive();
    let day_start_time = now.and_hms_opt(0, 0, 0).unwrap();
    let (cycle_day_uplink_traffic_usage, cycle_day_downlink_traffic_usage) =
        monitor_day_mapper::sum_daterange_data(
            current_cycle_start_date,
            chrono::Local::now().date_naive() - chrono::Duration::days(1),
            &app_state.db_pool,
        )
        .await?
        .unwrap_or((0, 0));
    let (today_uplink_traffic_usage, today_downlink_traffic_usage) =
        monitor_second_mapper::sum_timerange_data(
            day_start_time,
            day_start_time + Duration::days(1),
            &app_state.db_pool,
        )
        .await?
        .unwrap_or((0, 0));
    let uplink_traffic_usage = cycle_day_uplink_traffic_usage + today_uplink_traffic_usage;
    let downlink_traffic_usage = cycle_day_downlink_traffic_usage + today_downlink_traffic_usage;
    let traffic_usage = match statistic_method {
        CycleStatisticMethod::MaxInOut => {
            std::cmp::max(uplink_traffic_usage, downlink_traffic_usage)
        }
        CycleStatisticMethod::OnlyOut => uplink_traffic_usage,
        CycleStatisticMethod::SumInOut => uplink_traffic_usage + downlink_traffic_usage,
    };
    let mut cycle_notify_list = vec![];
    if let Some(notify) = &liftcycle.notify {
        let mut notify = notify.clone();
        notify.sort_by(|a, b| b.percent.cmp(&a.percent));
        for ele in notify {
            cycle_notify_list.push(CycleNotifyAppState {
                percent: ele.percent,
                exec: ele.exec,
                finished: false,
            });
        }
    }
    let cycle = CycleAppState {
        cycle_type,
        current_cycle_start_date,
        current_cycle_end_date,
        uplink_traffic_usage,
        downlink_traffic_usage,
        traffic_limit,
        traffic_usage,
        notify: cycle_notify_list,
        statistic_method,
    };
    tracing::info!("流量周期: {:#?}", &cycle);
    *app_state.cycle.write().await = Some(cycle);
    anyhow::Ok(())
}
