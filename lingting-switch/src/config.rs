use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const FRAMEWORK_KEY: &str = "LINGTING_SWITCH_FRAMEWORK";
const LRI_KEY: &str = "LINGTING_SWITCH_LRI";
const FRAMEWORK_DEFAULT: &str = r"D:\code\lingting\lingting-rust-framework";
const LRI_DEFAULT: &str = r"D:\code\lingting\lingting-react-ui\src";

/// 本地 framework 与 lri 的目录
#[derive(Debug, Clone)]
pub struct LocalDirectorys {
    pub framework: PathBuf,
    pub lri: PathBuf,
}

impl LocalDirectorys {
    /// lri 的 package.json 位于 LRI 目录的上一级
    pub fn lri_package_json(&self) -> PathBuf {
        let parent = self.lri.parent().unwrap_or(self.lri.as_path());
        parent.join("package.json")
    }
}

/// 获取环境变量或默认路径，并转换为规范化后的绝对路径
pub fn local_directorys() -> Result<LocalDirectorys> {
    Ok(LocalDirectorys {
        framework: convert(FRAMEWORK_KEY, &source(FRAMEWORK_KEY, FRAMEWORK_DEFAULT))?,
        lri: convert(LRI_KEY, &source(LRI_KEY, LRI_DEFAULT))?,
    })
}

fn source(key: &str, default: &str) -> String {
    match env::var(key) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_string(),
    }
}

fn convert(key: &str, source: &str) -> Result<PathBuf> {
    let trim = source.trim();
    if trim.is_empty() {
        bail!("[{key}] 路径值异常!");
    }

    let path = PathBuf::from(trim);
    let canonicalize = path
        .canonicalize()
        .with_context(|| format!("[{key}] 该路径不存在: {trim}"))?;

    if !is_directory(&canonicalize) {
        bail!("[{key}] 不是目录: {trim}");
    }

    let simplified = simplify(&canonicalize);
    println!("[{key}] 使用路径: {}", simplified.display());
    Ok(simplified)
}

/// 去掉 Windows 扩展长度前缀，避免把 \\?\ 写入配置或链接目标
fn simplify(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }

    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path.to_path_buf(),
    }
}

fn is_directory(path: &Path) -> bool {
    path.is_dir()
}
