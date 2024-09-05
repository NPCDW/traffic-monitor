use anyhow::Ok;
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::{config::state::AppState, service::statistics_svc};

pub async fn init(app_state: &AppState) -> anyhow::Result<()> {
    let sched = JobScheduler::new().await?;
    
    let app_state_clone = app_state.clone();
    tracing::info!("开始收集监控数据");
    sched.add(Job::new_cron_job_async_tz("0/15 * * * * ? ", chrono::Local, move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let res = statistics_svc::collect_second_data(&app_state).await;
            if res.is_err() {
                tracing::error!("收集秒级监控数据出错: {:?}", &res);
            }
        })
    })?).await?;

    let app_state_clone = app_state.clone();
    sched.add(Job::new_cron_job_async_tz("0 1 * * * ? ", chrono::Local, move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let statistic_hour_time = chrono::Local::now() - chrono::Duration::hours(1);
            let res = statistics_svc::collect_hour_data(&app_state, statistic_hour_time.naive_local()).await;
            if res.is_err() {
                tracing::error!("收集小时监控数据出错: {:?}", &res);
            }
        })
    })?).await?;

    let app_state_clone = app_state.clone();
    sched.add(Job::new_cron_job_async_tz("0 2 0 * * ? ", chrono::Local, move |_uuid, _l| {
        let app_state = app_state_clone.clone();
        Box::pin(async move {
            let statistic_date = chrono::Local::now() - chrono::Duration::days(1);
            let res = statistics_svc::collect_day_data(&app_state, statistic_date.date_naive()).await;
            if res.is_err() {
                tracing::error!("收集天监控数据出错: {:?}", &res);
            }
        })
    })?).await?;

    sched.start().await?;
    Ok(())
}
