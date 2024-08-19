use serde_json::json;

use crate::util::http_util;

pub async fn send_msg(config: &crate::config::app_config::Config, text: String) {
    if let Some(tg) = &config.tg {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", tg.bot_token);
        let body = json!({"chat_id": tg.chat_id, "text": text, "parse_mode": "Markdown", "message_thread_id": tg.topic_id}).to_string();
        tracing::debug!("tg 发送消息 body: {}", &body);
        match http_util::post(&url, body).await {
            Ok(_) => tracing::info!("tg 消息发送成功"),
            Err(e) => tracing::error!("tg 消息发送失败: {}", e),
        }
    }
}
