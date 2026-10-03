use std::path::Path;

use anyhow::{Context, Result, bail};
use toml_edit::{DocumentMut, InlineTable, Item, TableLike, Value};

use crate::mode::Mode;
use crate::plan::RustDepPlan;
use crate::text_file;

use super::framework;

/// Cargo.toml 中直接声明依赖的表
const TABLES: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// 一个参与切换的 framework 依赖
#[derive(Debug, Clone)]
pub struct FrameworkDep {
    pub table: Vec<String>,
    pub key: String,
    pub name: String,
    pub mode: Mode,
}

/// 扫描 Cargo.toml 中的 framework 依赖
pub fn scan(path: &Path) -> Result<Vec<FrameworkDep>> {
    let document = framework::read_document(path)?;

    let mut deps = vec![];
    for (table, item) in dep_tables(&document) {
        let Some(table_like) = item.as_table_like() else {
            continue;
        };

        for (key, value) in table_like.iter() {
            if !is_framework(key, value) {
                continue;
            }

            // workspace = true 由 workspace 层声明，不参与切换
            if value.get("workspace").and_then(Item::as_bool) == Some(true) {
                continue;
            }

            let name = value.get("package").and_then(Item::as_str).unwrap_or(key);
            deps.push(FrameworkDep {
                table: table.clone(),
                key: key.to_string(),
                name: name.to_string(),
                mode: dep_mode(key, value)?,
            });
        }
    }

    Ok(deps)
}

/// 按目标模式改写 Cargo.toml
pub fn apply(path: &Path, target: Mode, deps: &[RustDepPlan]) -> Result<()> {
    let mut document = framework::read_document(path)?;
    for dep in deps {
        let item = entry_mut(&mut document, dep)?;
        match target {
            // 切换为本地模式：删除 version，写入 path
            Mode::Local => {
                let table = as_table(item, dep)?;
                table.remove("version");
                table.insert("path", Value::from(dep.value.as_str()).into());
            }
            // 切换为依赖模式：删除 path，写入 version
            Mode::Dependency => {
                let table = as_table(item, dep)?;
                table.remove("path");
                table.insert("version", Value::from(dep.value.as_str()).into());
            }
        }
    }

    text_file::write(path, &document.to_string())
}

/// 依赖的当前模式：path 优先，其次 version
fn dep_mode(key: &str, value: &Item) -> Result<Mode> {
    if value.get("path").is_some() {
        return Ok(Mode::Local);
    }

    if value.get("version").is_some() || value.is_str() {
        return Ok(Mode::Dependency);
    }

    bail!("依赖 {key} 未声明 path 或 version，无法识别模式")
}

/// 依赖键名或 package 字段指向 framework
fn is_framework(key: &str, value: &Item) -> bool {
    if is_framework_name(key) {
        return true;
    }

    value
        .get("package")
        .and_then(Item::as_str)
        .is_some_and(is_framework_name)
}

fn is_framework_name(name: &str) -> bool {
    name == "framework" || name.starts_with("framework-")
}

/// 依赖直接声明的表、workspace 层以及 target 层下的表
fn dep_tables(document: &DocumentMut) -> Vec<(Vec<String>, &Item)> {
    let mut tables = vec![];

    for name in TABLES {
        if let Some(item) = document.get(name) {
            tables.push((vec![name.to_string()], item));
        }
    }

    if let Some(item) = document
        .get("workspace")
        .and_then(|item| item.get("dependencies"))
    {
        tables.push((
            vec!["workspace".to_string(), "dependencies".to_string()],
            item,
        ));
    }

    if let Some(target) = document.get("target").and_then(Item::as_table) {
        for (configuration, item) in target.iter() {
            for name in TABLES {
                if let Some(dependency) = item.get(name) {
                    tables.push((
                        vec![
                            "target".to_string(),
                            configuration.to_string(),
                            name.to_string(),
                        ],
                        dependency,
                    ));
                }
            }
        }
    }

    tables
}

/// 定位依赖的可写条目
fn entry_mut<'a>(document: &'a mut DocumentMut, dep: &RustDepPlan) -> Result<&'a mut Item> {
    let mut current = document.as_item_mut();
    for name in dep.table.iter().chain(std::iter::once(&dep.key)) {
        current = current
            .get_mut(name.as_str())
            .with_context(|| format!("Cargo.toml 中未找到 {name}"))?;
    }

    Ok(current)
}

/// 取得依赖的可写表，字符串简写先展开为内联表
fn as_table<'a>(item: &'a mut Item, dep: &RustDepPlan) -> Result<&'a mut dyn TableLike> {
    if item.is_str() {
        let version = item.as_str().unwrap_or_default().to_string();
        let mut table = InlineTable::new();
        table.insert("version", Value::from(version));
        *item = Item::Value(Value::InlineTable(table));
    }

    item.as_table_like_mut()
        .with_context(|| format!("依赖 {} 不是可修改的表", dep.key))
}
