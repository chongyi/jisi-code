use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const METHOD_INITIALIZE: &str = "acp/initialize";
pub const METHOD_SEND_MESSAGE: &str = "acp/sendMessage";
pub const METHOD_CANCEL: &str = "acp/cancelRequest";
pub const NOTIF_CONTENT_DELTA: &str = "acp/contentDelta";
pub const NOTIF_TOOL_CALL: &str = "acp/toolCall";
pub const NOTIF_STATUS: &str = "acp/statusUpdate";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
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
