//! 文件系统 API 路由。
//!
//! 提供目录浏览、文件搜索等能力给前端组件使用。

use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use system_capabilities::{
    DirectoryInfo, FileSystemEntry, FileSystemError, SearchOptions, SearchResult,
};

use super::state::AppState;

/// 创建文件系统 API 路由。
pub fn create_filesystem_router() -> Router<Arc<AppState>> {
    Router::new()
        // 列出目录内容
        .route("/api/fs/list", get(list_directory))
        // 获取常见目录
        .route("/api/fs/common", get(get_common_directories))
        // 获取当前工作目录
        .route("/api/fs/cwd", get(get_current_directory))
        // 获取主目录
        .route("/api/fs/home", get(get_home_directory))
        // 检查路径是否存在
        .route("/api/fs/exists/{*path}", get(path_exists))
        // 搜索文件
        .route("/api/fs/search", get(search_files))
        // 获取目录信息（通过路径参数）
        .route("/api/fs/dir/{*path}", get(get_directory_info))
}

/// 列出目录内容查询参数。
#[derive(Debug, Deserialize)]
struct ListDirectoryQuery {
    /// 目录路径。
    path: String,
}

/// 列出目录内容。
async fn list_directory(
    state: axum::extract::State<Arc<AppState>>,
    Query(query): Query<ListDirectoryQuery>,
) -> Result<Json<DirectoryInfo>, ApiError> {
    let info = state.filesystem.list_directory(&query.path)?;
    Ok(Json(info))
}

/// 获取目录信息（通过路径参数）。
async fn get_directory_info(
    state: axum::extract::State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<DirectoryInfo>, ApiError> {
    let info = state.filesystem.list_directory(&path)?;
    Ok(Json(info))
}

/// 获取常见目录列表。
async fn get_common_directories(
    state: axum::extract::State<Arc<AppState>>,
) -> Json<Vec<FileSystemEntry>> {
    Json(state.filesystem.get_common_directories())
}

/// 获取当前工作目录。
async fn get_current_directory(
    state: axum::extract::State<Arc<AppState>>,
) -> Result<Json<CurrentDirectoryResponse>, ApiError> {
    let cwd = state.filesystem.get_current_directory()?;
    Ok(Json(CurrentDirectoryResponse { path: cwd }))
}

#[derive(Debug, Serialize)]
struct CurrentDirectoryResponse {
    path: String,
}

/// 获取用户主目录。
async fn get_home_directory(state: axum::extract::State<Arc<AppState>>) -> Json<Option<String>> {
    Json(state.filesystem.get_home_directory())
}

/// 检查路径是否存在。
async fn path_exists(
    state: axum::extract::State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Json<PathExistsResponse> {
    let exists = state.filesystem.path_exists(&path);
    let is_dir = state.filesystem.is_directory(&path);
    Json(PathExistsResponse {
        exists,
        is_dir,
        is_file: exists && !is_dir,
    })
}

#[derive(Debug, Serialize)]
struct PathExistsResponse {
    exists: bool,
    is_dir: bool,
    is_file: bool,
}

/// 搜索文件查询参数。
#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// 基础路径。
    base_path: String,
    /// 搜索模式（glob 格式）。
    pattern: String,
    /// 是否递归搜索。
    #[serde(default = "default_recursive")]
    recursive: bool,
    /// 是否包含隐藏文件。
    #[serde(default)]
    include_hidden: bool,
    /// 最大搜索深度。
    #[serde(default = "default_max_depth")]
    max_depth: usize,
    /// 最大结果数量。
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_recursive() -> bool {
    true
}

fn default_max_depth() -> usize {
    10
}

fn default_max_results() -> usize {
    100
}

/// 搜索文件。
async fn search_files(
    state: axum::extract::State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResult>, ApiError> {
    let options = SearchOptions {
        pattern: query.pattern,
        recursive: query.recursive,
        include_hidden: query.include_hidden,
        max_depth: query.max_depth,
        max_results: query.max_results,
    };
    let result = state.filesystem.search_files(&query.base_path, &options)?;
    Ok(Json(result))
}

/// API 错误响应。
#[derive(Debug, Serialize)]
struct ApiErrorResponse {
    error: String,
    code: String,
}

/// API 错误类型。
#[derive(Debug)]
struct ApiError {
    message: String,
    code: String,
    status: StatusCode,
}

impl From<FileSystemError> for ApiError {
    fn from(err: FileSystemError) -> Self {
        match err {
            FileSystemError::PathNotFound(path) => ApiError {
                message: format!("Path not found: {}", path),
                code: "PATH_NOT_FOUND".to_string(),
                status: StatusCode::NOT_FOUND,
            },
            FileSystemError::NotADirectory(path) => ApiError {
                message: format!("Not a directory: {}", path),
                code: "NOT_A_DIRECTORY".to_string(),
                status: StatusCode::BAD_REQUEST,
            },
            FileSystemError::PermissionDenied(path) => ApiError {
                message: format!("Permission denied: {}", path),
                code: "PERMISSION_DENIED".to_string(),
                status: StatusCode::FORBIDDEN,
            },
            FileSystemError::Io(e) => ApiError {
                message: format!("IO error: {}", e),
                code: "IO_ERROR".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            FileSystemError::Other(e) => ApiError {
                message: e.to_string(),
                code: "INTERNAL_ERROR".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ApiErrorResponse {
            error: self.message,
            code: self.code,
        });
        (self.status, body).into_response()
    }
}
