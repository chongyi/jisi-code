//! System Capabilities - 系统能力封装模块。
//!
//! 该 crate 提供统一的系统能力接口，供 server 集成为 API 路由，
//! 为前端组件提供必要的系统能力支持。

pub mod filesystem;

pub use filesystem::{
    DirectoryInfo, FileInfo, FileSystemCapabilities, FileSystemEntry, FileSystemError,
    SearchOptions, SearchResult,
};
