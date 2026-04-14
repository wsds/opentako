use serde::{Deserialize, Serialize};

// 客户端发给引擎的请求
#[derive(Deserialize, Debug)]
pub struct WsRequest {
    pub task_id: String,
    pub action: String,
    pub payload: Payload,
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub text: String,
    pub model_id: Option<String>,
    pub speed: Option<f32>,
}

// 引擎返回给客户端的事件
#[derive(Serialize, Debug)]
pub struct WsEvent {
    pub task_id: String,
    pub event: String,
    // 使用 Option 并且 skip_serializing_if 可以在值为 None 时不在 JSON 中输出该字段
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u32>,
}