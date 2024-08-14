use serde::{Serialize, Deserialize};
use crate::util::file_util;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CycleConfig {
    pub cycle: String,
    pub each: Option<u32>,
    pub traffic_reset_date: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub traffic_limit: String,
    pub statistic_method: String,
    pub exec: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TgConfig {
    pub bot_token: String,
    pub chat_id: String,
    pub topic_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub network_name: String,
    pub log_level: String,
    pub vps_name: String,
    pub tg: Option<TgConfig>,
    pub liftcycle: Option<CycleConfig>,
}

const CONFIG_FILE_NAME: &'static str = "config/config.json";

pub fn get_config() -> Config {
    let current_dir = file_util::get_current_dir();
    let filepath = current_dir.join(CONFIG_FILE_NAME);
    if !filepath.exists() {
        tracing::error!("没有在工作目录 {:?} 找到 {:?}", current_dir, CONFIG_FILE_NAME);
        std::process::exit(1);
    }
    let buf = file_util::read_file(&filepath).unwrap_or_else(|e| {
        panic!("读取配置文件失败: {}, {:?}", &filepath.display() ,e);
    });
    let config: Config = serde_json::from_str(&buf).unwrap_or_else(|e| {
        panic!("配置文件 {} 可能不是 json 格式: {:?}", &filepath.display(), e);
    });
    config
}
