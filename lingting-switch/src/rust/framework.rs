use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use toml_edit::{DocumentMut, Item};

use crate::text_file;

/// 校验 framework 包存在且包名一致，返回包目录
pub fn ensure_package(framework: &Path, name: &str) -> Result<PathBuf> {
    let directory = framework.join(name);
    let file = directory.join("Cargo.toml");
    if !file.is_file() {
        bail!("framework 中未找到依赖 {name}: {}", file.display());
    }

    let document = read_document(&file)?;
    let package_name = package_value(&document, "name")
        .with_context(|| format!("{} 缺少 [package].name", file.display()))?;
    if package_name != name {
        bail!(
            "{} 的包名与依赖名不一致: {package_name} != {name}",
            file.display()
        );
    }

    Ok(directory)
}

/// framework 包对应的版本号
pub fn version(framework: &Path, name: &str) -> Result<String> {
    let file = ensure_package(framework, name)?.join("Cargo.toml");
    let document = read_document(&file)?;
    if let Some(version) = package_value(&document, "version") {
        return Ok(version);
    }

    // version.workspace = true 时版本来自 framework 根 Cargo.toml
    let root = framework.join("Cargo.toml");
    let document = read_document(&root)?;
    document
        .get("workspace")
        .and_then(Item::as_table)
        .and_then(|table| table.get("package"))
        .and_then(Item::as_table)
        .and_then(|table| table.get("version"))
        .and_then(Item::as_str)
        .map(str::to_string)
        .with_context(|| format!("未从 {} 获取 {name} 的版本", root.display()))
}

/// 读取并解析 TOML 文档
pub fn read_document(path: &Path) -> Result<DocumentMut> {
    let text = text_file::read(path)?;
    text.parse::<DocumentMut>()
        .with_context(|| format!("TOML 解析失败: {}", path.display()))
}

fn package_value(document: &DocumentMut, key: &str) -> Option<String> {
    document
        .get("package")
        .and_then(Item::as_table)
        .and_then(|table| table.get(key))
        .and_then(Item::as_str)
        .map(str::to_string)
}
