use serde::{Deserialize, Serialize};
use serde_json::Value;

/// ACP 初始化方法名。
pub const METHOD_INITIALIZE: &str = "acp/initialize";
/// ACP 发送消息方法名。
pub const METHOD_SEND_MESSAGE: &str = "acp/sendMessage";
/// ACP 取消请求方法名。
pub const METHOD_CANCEL: &str = "acp/cancelRequest";
/// ACP 内容增量通知方法名。
pub const NOTIF_CONTENT_DELTA: &str = "acp/contentDelta";
/// ACP 工具调用通知方法名。
pub const NOTIF_TOOL_CALL: &str = "acp/toolCall";
/// ACP 状态更新通知方法名。
pub const NOTIF_STATUS: &str = "acp/statusUpdate";

/// JSON-RPC 请求对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// 协议版本，固定为 `"2.0"`。
    pub jsonrpc: String,
    /// 请求 ID。
    pub id: u64,
    /// 请求方法名。
    pub method: String,
    /// 请求参数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// 构造 JSON-RPC 请求。
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 响应对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// 协议版本，通常为 `"2.0"`。
    pub jsonrpc: String,
    /// 对应请求 ID。
    pub id: u64,
    /// 成功结果。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 失败错误。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 错误对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// 错误码。
    pub code: i32,
    /// 错误消息。
    pub message: String,
    /// 附加错误数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 通知对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// 协议版本，通常为 `"2.0"`。
    pub jsonrpc: String,
    /// 通知方法名。
    pub method: String,
    /// 通知参数。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_request() {
        let request = JsonRpcRequest::new(1, METHOD_INITIALIZE, Some(json!({ "client": "test" })));

        let serialized = serde_json::to_value(&request).expect("request should serialize");

        assert_eq!(serialized["jsonrpc"], "2.0");
        assert_eq!(serialized["id"], 1);
        assert_eq!(serialized["method"], METHOD_INITIALIZE);
        assert_eq!(serialized["params"]["client"], "test");
    }

    #[test]
    fn test_deserialize_notification() {
        let raw = json!({
            "jsonrpc": "2.0",
            "method": NOTIF_STATUS,
            "params": {
                "status": "running"
            }
        });

        let notification: JsonRpcNotification =
            serde_json::from_value(raw).expect("notification should deserialize");

        assert_eq!(notification.jsonrpc, "2.0");
        assert_eq!(notification.method, NOTIF_STATUS);
        assert_eq!(
            notification
                .params
                .as_ref()
                .and_then(|p| p.get("status"))
                .and_then(|v| v.as_str()),
            Some("running")
        );
    }
}
