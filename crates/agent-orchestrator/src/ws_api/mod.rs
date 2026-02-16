//! WebSocket API 模块，仅在 `ws-api` feature 启用时可用。

mod adapter;
mod handler;
mod protocol;

pub use adapter::event_to_server_message;
pub use handler::websocket_handler;
pub use protocol::{AgentInfoMessage, ClientMessage, ServerMessage, SessionInfoMessage};
