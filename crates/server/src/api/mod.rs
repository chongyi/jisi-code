//! API 路由模块。
//!
//! 提供前端组件所需的系统能力 API。

pub mod filesystem;
pub mod state;
pub mod ws;

pub use filesystem::create_filesystem_router;
pub use state::AppState;
pub use ws::websocket_handler;
