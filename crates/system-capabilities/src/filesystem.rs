//! 文件系统能力模块。
//!
//! 提供文件系统访问、目录浏览、文件搜索等能力。

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;
use walkdir::WalkDir;

/// 文件系统错误类型。
#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("路径不存在: {0}")]
    PathNotFound(String),

    #[error("路径不是目录: {0}")]
    NotADirectory(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, FileSystemError>;

/// 文件/目录信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemEntry {
    /// 名称。
    pub name: String,
    /// 完整路径。
    pub path: String,
    /// 是否为目录。
    pub is_dir: bool,
    /// 是否为文件。
    pub is_file: bool,
    /// 是否为符号链接。
    pub is_symlink: bool,
    /// 文件大小（字节），仅对文件有效。
    pub size: Option<u64>,
    /// 修改时间（Unix 时间戳）。
    pub modified: Option<u64>,
    /// 是否为隐藏文件/目录。
    pub is_hidden: bool,
}

/// 目录详细内容信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// 目录路径。
    pub path: String,
    /// 目录名称。
    pub name: String,
    /// 父目录路径。
    pub parent: Option<String>,
    /// 子目录列表。
    pub directories: Vec<FileSystemEntry>,
    /// 文件列表。
    pub files: Vec<FileSystemEntry>,
    /// 是否可以访问（权限）。
    pub accessible: bool,
}

/// 文件信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// 文件路径。
    pub path: String,
    /// 文件名称。
    pub name: String,
    /// 文件大小（字节）。
    pub size: u64,
    /// 修改时间（Unix 时间戳）。
    pub modified: Option<u64>,
    /// 文件扩展名。
    pub extension: Option<String>,
    /// 是否为文本文件（基于扩展名猜测）。
    pub is_text: bool,
}

/// 文件搜索选项。
#[derive(Debug, Clone, Deserialize)]
pub struct SearchOptions {
    /// 搜索模式（glob 格式）。
    pub pattern: String,
    /// 是否递归搜索。
    #[serde(default = "default_recursive")]
    pub recursive: bool,
    /// 是否包含隐藏文件。
    #[serde(default)]
    pub include_hidden: bool,
    /// 最大搜索深度。
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    /// 最大结果数量。
    #[serde(default = "default_max_results")]
    pub max_results: usize,
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

/// 搜索结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 匹配的文件列表。
    pub files: Vec<FileSystemEntry>,
    /// 总匹配数（可能被截断）。
    pub total: usize,
    /// 是否被截断。
    pub truncated: bool,
}

/// 文件系统能力接口。
#[derive(Clone)]
pub struct FileSystemCapabilities {
    /// 允许的根目录列表（用于安全限制）。
    allowed_roots: Vec<PathBuf>,
}

impl FileSystemCapabilities {
    /// 创建新的文件系统能力实例。
    pub fn new() -> Self {
        Self {
            allowed_roots: vec![PathBuf::from("/")],
        }
    }

    /// 创建带有根目录限制的实例。
    pub fn with_allowed_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            allowed_roots: if roots.is_empty() {
                vec![PathBuf::from("/")]
            } else {
                roots
            },
        }
    }

    /// 检查路径是否在允许范围内。
    fn is_path_allowed(&self, path: &Path) -> bool {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        self.allowed_roots
            .iter()
            .any(|root| canonical.starts_with(root))
    }

    /// 列出目录内容。
    pub fn list_directory(&self, path: &str) -> Result<DirectoryInfo> {
        let path = PathBuf::from(path);

        if !path.exists() {
            return Err(FileSystemError::PathNotFound(path.display().to_string()));
        }

        if !path.is_dir() {
            return Err(FileSystemError::NotADirectory(path.display().to_string()));
        }

        if !self.is_path_allowed(&path) {
            return Err(FileSystemError::PermissionDenied(
                path.display().to_string(),
            ));
        }

        info!(path = %path.display(), "Listing directory");

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let parent = path.parent().map(|p| p.display().to_string());

        let mut directories = Vec::new();
        let mut files = Vec::new();
        let mut accessible = true;

        match std::fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(entry_info) = self.entry_to_info(&entry) {
                        if entry_info.is_dir {
                            directories.push(entry_info);
                        } else {
                            files.push(entry_info);
                        }
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    accessible = false;
                }
            }
        }

        // 排序：目录优先，然后按名称排序
        directories.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(DirectoryInfo {
            path: path.display().to_string(),
            name,
            parent,
            directories,
            files,
            accessible,
        })
    }

    /// 将目录条目转换为信息结构。
    fn entry_to_info(&self, entry: &std::fs::DirEntry) -> Result<FileSystemEntry> {
        let path = entry.path();
        let metadata = entry.metadata()?;

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let is_hidden = name.starts_with('.');

        let modified = metadata.modified().ok().and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs())
        });

        Ok(FileSystemEntry {
            name,
            path: path.display().to_string(),
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: metadata.file_type().is_symlink(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified,
            is_hidden,
        })
    }

    /// 获取用户主目录。
    pub fn get_home_directory(&self) -> Option<String> {
        dirs::home_dir().map(|p| p.display().to_string())
    }

    /// 获取当前工作目录。
    pub fn get_current_directory(&self) -> Result<String> {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .map_err(FileSystemError::Io)
    }

    /// 获取常见目录列表（主目录、桌面、文档等）。
    pub fn get_common_directories(&self) -> Vec<FileSystemEntry> {
        let mut dirs = Vec::new();

        // 主目录
        if let Some(home) = dirs::home_dir() {
            dirs.push(FileSystemEntry {
                name: "Home".to_string(),
                path: home.display().to_string(),
                is_dir: true,
                is_file: false,
                is_symlink: false,
                size: None,
                modified: None,
                is_hidden: false,
            });
        }

        // 桌面
        if let Some(desktop) = dirs::desktop_dir() {
            dirs.push(FileSystemEntry {
                name: "Desktop".to_string(),
                path: desktop.display().to_string(),
                is_dir: true,
                is_file: false,
                is_symlink: false,
                size: None,
                modified: None,
                is_hidden: false,
            });
        }

        // 文档
        if let Some(documents) = dirs::document_dir() {
            dirs.push(FileSystemEntry {
                name: "Documents".to_string(),
                path: documents.display().to_string(),
                is_dir: true,
                is_file: false,
                is_symlink: false,
                size: None,
                modified: None,
                is_hidden: false,
            });
        }

        // 下载
        if let Some(downloads) = dirs::download_dir() {
            dirs.push(FileSystemEntry {
                name: "Downloads".to_string(),
                path: downloads.display().to_string(),
                is_dir: true,
                is_file: false,
                is_symlink: false,
                size: None,
                modified: None,
                is_hidden: false,
            });
        }

        // 项目目录（常见位置）
        let common_project_paths = if cfg!(target_os = "macos") {
            vec![
                dirs::home_dir().map(|h| h.join("Projects")),
                dirs::home_dir().map(|h| h.join("Developer")),
                dirs::home_dir().map(|h| h.join("workspace")),
                dirs::home_dir().map(|h| h.join("code")),
            ]
        } else if cfg!(target_os = "windows") {
            vec![
                dirs::home_dir().map(|h| h.join("Projects")),
                dirs::home_dir().map(|h| h.join("source")),
                dirs::home_dir().map(|h| h.join("code")),
            ]
        } else {
            vec![
                dirs::home_dir().map(|h| h.join("Projects")),
                dirs::home_dir().map(|h| h.join("workspace")),
                dirs::home_dir().map(|h| h.join("code")),
            ]
        };

        for path_opt in common_project_paths {
            if let Some(path) = path_opt {
                if path.exists() && path.is_dir() {
                    dirs.push(FileSystemEntry {
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string()),
                        path: path.display().to_string(),
                        is_dir: true,
                        is_file: false,
                        is_symlink: false,
                        size: None,
                        modified: None,
                        is_hidden: false,
                    });
                }
            }
        }

        dirs
    }

    /// 搜索文件。
    pub fn search_files(&self, base_path: &str, options: &SearchOptions) -> Result<SearchResult> {
        let base = PathBuf::from(base_path);

        if !base.exists() {
            return Err(FileSystemError::PathNotFound(base_path.to_string()));
        }

        if !self.is_path_allowed(&base) {
            return Err(FileSystemError::PermissionDenied(base_path.to_string()));
        }

        info!(base = %base_path, pattern = %options.pattern, "Searching files");

        let mut files = Vec::new();
        let mut total = 0;

        let pattern = glob::Pattern::new(&options.pattern)
            .with_context(|| format!("Invalid glob pattern: {}", options.pattern))
            .map_err(FileSystemError::Other)?;

        let include_hidden = options.include_hidden;

        for entry in WalkDir::new(&base)
            .max_depth(if options.recursive {
                options.max_depth
            } else {
                1
            })
            .into_iter()
            .filter_entry(|e| {
                if !include_hidden {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.')
                } else {
                    true
                }
            })
        {
            if files.len() >= options.max_results {
                total += 1;
                continue;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();

            // 检查是否匹配模式
            let relative = path.strip_prefix(&base).unwrap_or(path);
            if pattern.matches_path(relative) || pattern.matches_path(path) {
                if let Ok(metadata) = entry.metadata() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let is_hidden = name.starts_with('.');

                    let modified = metadata.modified().ok().and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs())
                    });

                    files.push(FileSystemEntry {
                        name,
                        path: path.display().to_string(),
                        is_dir: metadata.is_dir(),
                        is_file: metadata.is_file(),
                        is_symlink: metadata.file_type().is_symlink(),
                        size: if metadata.is_file() {
                            Some(metadata.len())
                        } else {
                            None
                        },
                        modified,
                        is_hidden,
                    });
                    total += 1;
                }
            }
        }

        let truncated = files.len() >= options.max_results;
        Ok(SearchResult {
            files,
            total,
            truncated,
        })
    }

    /// 检查路径是否存在。
    pub fn path_exists(&self, path: &str) -> bool {
        PathBuf::from(path).exists()
    }

    /// 检查路径是否为目录。
    pub fn is_directory(&self, path: &str) -> bool {
        PathBuf::from(path).is_dir()
    }

    /// 获取路径的规范形式。
    pub fn canonicalize(&self, path: &str) -> Result<String> {
        PathBuf::from(path)
            .canonicalize()
            .map(|p| p.display().to_string())
            .map_err(FileSystemError::Io)
    }
}

impl Default for FileSystemCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_directory() {
        let fs = FileSystemCapabilities::new();
        let result = fs.list_directory(".");
        assert!(result.is_ok());
        let info = result.unwrap();
        assert!(!info.directories.is_empty() || !info.files.is_empty());
    }

    #[test]
    fn test_get_common_directories() {
        let fs = FileSystemCapabilities::new();
        let dirs = fs.get_common_directories();
        assert!(!dirs.is_empty());
    }
}
