import {
  ChevronLeft,
  ChevronRight,
  Folder,
  FolderOpen,
  HardDrive,
  Home,
  Loader2,
  RefreshCw,
  Search,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";

import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { cn } from "~/lib/utils";
import type { DirectoryInfo, FileSystemEntry } from "~/types/filesystem";
import { getCommonDirectories, getCurrentDirectory, listDirectory } from "~/types/filesystem";

interface DirectoryPickerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (path: string) => void;
  initialPath?: string;
  title?: string;
}

export function DirectoryPicker({
  open,
  onOpenChange,
  onSelect,
  initialPath,
  title = "Select Directory",
}: DirectoryPickerProps) {
  const [currentPath, setCurrentPath] = useState(initialPath || "");
  const [directoryInfo, setDirectoryInfo] = useState<DirectoryInfo | null>(null);
  const [commonDirs, setCommonDirs] = useState<FileSystemEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchFilter, setSearchFilter] = useState("");

  // 加载常见目录
  useEffect(() => {
    if (open) {
      getCommonDirectories().then(setCommonDirs).catch(console.error);
      if (!currentPath) {
        getCurrentDirectory()
          .then((cwd) => {
            setCurrentPath(cwd.path);
            loadDirectory(cwd.path);
          })
          .catch(console.error);
      }
    }
  }, [open]);

  // 加载目录内容
  const loadDirectory = useCallback(async (path: string) => {
    setIsLoading(true);
    setError(null);
    try {
      const info = await listDirectory(path);
      setDirectoryInfo(info);
      setCurrentPath(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load directory");
      setDirectoryInfo(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // 处理目录双击进入
  const handleDirectoryDoubleClick = (entry: FileSystemEntry) => {
    if (entry.is_dir) {
      loadDirectory(entry.path);
    }
  };

  // 处理返回上一级
  const handleGoUp = () => {
    if (directoryInfo?.parent) {
      loadDirectory(directoryInfo.parent);
    }
  };

  // 处理常见目录点击
  const handleCommonDirClick = (entry: FileSystemEntry) => {
    loadDirectory(entry.path);
  };

  // 处理确认选择
  const handleConfirm = () => {
    onSelect(currentPath);
    onOpenChange(false);
  };

  // 过滤目录列表
  const filteredDirectories = directoryInfo?.directories.filter((dir) =>
    dir.name.toLowerCase().includes(searchFilter.toLowerCase())
  ) ?? [];

  const filteredFiles = directoryInfo?.files.filter((file) =>
    file.name.toLowerCase().includes(searchFilter.toLowerCase())
  ) ?? [];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl h-[80vh] flex flex-col overflow-hidden">
        <DialogHeader className="shrink-0">
          <DialogTitle className="flex items-center gap-2">
            <FolderOpen className="size-5" />
            {title}
          </DialogTitle>
          <DialogDescription>
            Browse and select a directory for your project
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 min-h-0 flex flex-col gap-3 overflow-hidden">
          {/* 当前路径 */}
          <div className="flex items-center gap-2 shrink-0">
            <Input
              value={currentPath}
              onChange={(e) => setCurrentPath(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  loadDirectory(currentPath);
                }
              }}
              placeholder="Enter path..."
              className="flex-1"
            />
            <Button
              variant="outline"
              size="icon"
              onClick={() => loadDirectory(currentPath)}
              disabled={isLoading}
            >
              {isLoading ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <RefreshCw className="size-4" />
              )}
            </Button>
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive shrink-0">
              {error}
            </div>
          )}

          {/* 常见目录 */}
          {commonDirs.length > 0 && (
            <div className="flex flex-wrap gap-1.5 shrink-0">
              {commonDirs.map((dir) => (
                <Button
                  key={dir.path}
                  variant="outline"
                  size="sm"
                  className="h-7 text-xs"
                  onClick={() => handleCommonDirClick(dir)}
                >
                  {dir.name === "Home" ? (
                    <Home className="size-3 mr-1" />
                  ) : (
                    <Folder className="size-3 mr-1" />
                  )}
                  {dir.name}
                </Button>
              ))}
            </div>
          )}

          {/* 搜索过滤 */}
          <div className="relative shrink-0">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
            <Input
              value={searchFilter}
              onChange={(e) => setSearchFilter(e.target.value)}
              placeholder="Filter..."
              className="pl-8 h-8 text-sm"
            />
          </div>

          {/* 目录浏览区域 */}
          <div className="flex-1 min-h-0 border rounded-md overflow-auto">
            {isLoading ? (
              <div className="flex items-center justify-center py-20">
                <Loader2 className="size-8 animate-spin text-muted-foreground" />
              </div>
            ) : directoryInfo ? (
              <div className="p-2">
                {/* 返回上一级 */}
                {directoryInfo.parent && (
                  <button
                    type="button"
                    onClick={handleGoUp}
                    className="flex items-center gap-2 w-full px-2 py-1.5 rounded hover:bg-accent text-sm"
                  >
                    <ChevronLeft className="size-4" />
                    <span className="text-muted-foreground">..</span>
                  </button>
                )}

                {/* 目录列表 */}
                {filteredDirectories.map((dir) => (
                  <button
                    key={dir.path}
                    type="button"
                    onClick={() => setCurrentPath(dir.path)}
                    onDoubleClick={() => handleDirectoryDoubleClick(dir)}
                    className={cn(
                      "flex items-center gap-2 w-full px-2 py-1.5 rounded text-sm text-left",
                      "hover:bg-accent",
                      dir.path === currentPath && "bg-accent"
                    )}
                  >
                    <FolderOpen className="size-4 text-yellow-500 shrink-0" />
                    <span className="truncate">{dir.name}</span>
                    <ChevronRight className="size-4 ml-auto text-muted-foreground shrink-0" />
                  </button>
                ))}

                {/* 文件列表（仅显示，不可选择） */}
                {filteredFiles.length > 0 && (
                  <>
                    <div className="px-2 py-1.5 text-xs text-muted-foreground mt-2">
                      Files
                    </div>
                    {filteredFiles.slice(0, 20).map((file) => (
                      <div
                        key={file.path}
                        className="flex items-center gap-2 px-2 py-1.5 text-sm text-muted-foreground"
                      >
                        <HardDrive className="size-4 shrink-0" />
                        <span className="truncate">{file.name}</span>
                      </div>
                    ))}
                    {filteredFiles.length > 20 && (
                      <div className="px-2 py-1 text-xs text-muted-foreground">
                        +{filteredFiles.length - 20} more files
                      </div>
                    )}
                  </>
                )}

                {filteredDirectories.length === 0 && filteredFiles.length === 0 && (
                  <div className="flex items-center justify-center py-10 text-muted-foreground text-sm">
                    {searchFilter ? "No matching items" : "Empty directory"}
                  </div>
                )}
              </div>
            ) : (
              <div className="flex items-center justify-center py-20 text-muted-foreground">
                Enter a path to browse
              </div>
            )}
          </div>
        </div>

        <DialogFooter className="shrink-0">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleConfirm} disabled={!currentPath}>
            Select
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// 简化版目录选择器（用于 sidebar）
interface DirectoryInputProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  className?: string;
}

export function DirectoryInput({
  value,
  onChange,
  disabled,
  className,
}: DirectoryInputProps) {
  const [pickerOpen, setPickerOpen] = useState(false);

  return (
    <>
      <div className={cn("flex items-center gap-1", className)}>
        <Input
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder=". or /path/to/project"
          className="h-8 text-xs flex-1"
          disabled={disabled}
        />
        <Button
          variant="outline"
          size="icon"
          className="h-8 w-8 shrink-0"
          onClick={() => setPickerOpen(true)}
          disabled={disabled}
          title="Browse..."
        >
          <FolderOpen className="size-4" />
        </Button>
      </div>

      <DirectoryPicker
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        onSelect={onChange}
        initialPath={value}
        title="Select Working Directory"
      />
    </>
  );
}
