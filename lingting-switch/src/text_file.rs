use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// 读取文本文件
pub fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("无法读取文件: {}", path.display()))
}

/// 写出文本文件
pub fn write(path: &Path, text: &str) -> Result<()> {
    fs::write(path, text).with_context(|| format!("无法写入文件: {}", path.display()))
}

/// 判断文本使用的换行符
pub fn line_ending(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
