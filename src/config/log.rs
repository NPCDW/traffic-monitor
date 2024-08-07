use std::str::FromStr;

use time::UtcOffset;
use tracing_subscriber::{filter::LevelFilter, Layer, prelude::*, fmt::time::OffsetTime};

pub fn init(config: &crate::config::app_config::Config) {
    let local_time = OffsetTime::new(
        UtcOffset::from_hms(8, 0, 0).unwrap(),
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]").unwrap(),
    );
    let level = LevelFilter::from_str(&config.log_level).unwrap_or(LevelFilter::INFO);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_writer(std::io::stderr)
                .with_timer(local_time.clone())
                .with_filter(level)
        )
        .init();
}
