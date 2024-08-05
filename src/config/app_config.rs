use serde::{Serialize, Deserialize};
use crate::util::file_util;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthCheckConfig {
    pub interval_time: u64,
    pub connect_timeout: u64,
    pub test_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthServerConfig {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    pub web: String,
    #[serde(default)]
    pub auth: Option<AuthServerConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AirportInnerProxyConfig {
    pub url_type: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InnerProxyConfig {
    pub core: String,
    pub controller: String,
    pub secret: String,
    pub airport: AirportInnerProxyConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalProxyConfig {
    pub enabled: bool,
    pub external_controller: String,
    pub secret: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProxyConfig {
    pub enabled: bool,
    #[serde(default)]
    pub inner: Option<InnerProxyConfig>,
    #[serde(default)]
    pub external: Option<ExternalProxyConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub health_check: HealthCheckConfig,
    pub proxy: ProxyConfig,
}

const CONFIG_FILE_NAME: &'static str = "config/config.yml";

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
    let mut config: Config = serde_yaml::from_str(&buf).unwrap_or_else(|e| {
        panic!("配置文件 {} 可能不是 yaml 格式: {:?}", &filepath.display(), e);
    });
    if config.health_check.test_url.is_empty() {
        tracing::error!("测试地址不能为空");
        std::process::exit(1);
    }
    if !config.health_check.test_url.starts_with("http") {
        tracing::error!("测试地址请以 http 开头");
        std::process::exit(1);
    }
    if config.proxy.inner.is_none() && config.proxy.external.is_none() {
        tracing::error!("内置代理和外部代理请至少配置一个");
        std::process::exit(1);
    }
    if config.proxy.inner.is_some() && config.proxy.inner.as_ref().unwrap().secret.is_empty() {
        config.proxy.inner.as_mut().unwrap().secret = uuid::Uuid::new_v4().to_string()
    }
    config
}
