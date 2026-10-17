use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 收集运行目录及一级子目录中满足条件的目录，跳过隐藏目录
pub fn discover(startup: &Path, is_target: impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>> {
    let mut directories = vec![];
    if is_target(startup) {
        directories.push(startup.to_path_buf());
    }

    for entry in read_dir(startup)? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !entry.path().is_dir() {
            continue;
        }

        let directory = entry.path();
        if is_target(&directory) {
            directories.push(directory);
        }
    }

    Ok(directories)
}

/// 按文件名排序的目录项
pub fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = vec![];
    for entry in fs::read_dir(path).with_context(|| format!("无法读取目录: {}", path.display()))?
    {
        entries.push(entry.with_context(|| format!("无法读取目录项: {}", path.display()))?);
    }

    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}
