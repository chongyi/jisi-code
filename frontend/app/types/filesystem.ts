// 文件系统 API 类型定义

export interface FileSystemEntry {
  name: string;
  path: string;
  is_dir: boolean;
  is_file: boolean;
  is_symlink: boolean;
  size: number | null;
  modified: number | null;
  is_hidden: boolean;
}

export interface DirectoryInfo {
  path: string;
  name: string;
  parent: string | null;
  directories: FileSystemEntry[];
  files: FileSystemEntry[];
  accessible: boolean;
}

export interface SearchOptions {
  pattern: string;
  recursive?: boolean;
  include_hidden?: boolean;
  max_depth?: number;
  max_results?: number;
}

export interface SearchResult {
  files: FileSystemEntry[];
  total: number;
  truncated: boolean;
}

export interface PathExistsResponse {
  exists: boolean;
  is_dir: boolean;
  is_file: boolean;
}

export interface CurrentDirectoryResponse {
  path: string;
}

const API_BASE = "http://127.0.0.1:3001";

// API 客户端函数

export async function listDirectory(path: string): Promise<DirectoryInfo> {
  const response = await fetch(`${API_BASE}/api/fs/list?path=${encodeURIComponent(path)}`);
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || "Failed to list directory");
  }
  return response.json();
}

export async function getCommonDirectories(): Promise<FileSystemEntry[]> {
  const response = await fetch(`${API_BASE}/api/fs/common`);
  if (!response.ok) {
    throw new Error("Failed to get common directories");
  }
  return response.json();
}

export async function getCurrentDirectory(): Promise<CurrentDirectoryResponse> {
  const response = await fetch(`${API_BASE}/api/fs/cwd`);
  if (!response.ok) {
    throw new Error("Failed to get current directory");
  }
  return response.json();
}

export async function getHomeDirectory(): Promise<string | null> {
  const response = await fetch(`${API_BASE}/api/fs/home`);
  if (!response.ok) {
    throw new Error("Failed to get home directory");
  }
  return response.json();
}

export async function pathExists(path: string): Promise<PathExistsResponse> {
  const response = await fetch(`${API_BASE}/api/fs/exists/${encodeURIComponent(path)}`);
  if (!response.ok) {
    throw new Error("Failed to check path");
  }
  return response.json();
}

export async function searchFiles(
  basePath: string,
  options: SearchOptions
): Promise<SearchResult> {
  const params = new URLSearchParams({
    base_path: basePath,
    pattern: options.pattern,
    ...(options.recursive !== undefined && { recursive: String(options.recursive) }),
    ...(options.include_hidden !== undefined && { include_hidden: String(options.include_hidden) }),
    ...(options.max_depth !== undefined && { max_depth: String(options.max_depth) }),
    ...(options.max_results !== undefined && { max_results: String(options.max_results) }),
  });
  const response = await fetch(`${API_BASE}/api/fs/search?${params}`);
  if (!response.ok) {
    const error = await response.json();
    throw new Error(error.error || "Failed to search files");
  }
  return response.json();
}
