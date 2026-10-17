use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::common::command;
use crate::plan::RustPlanItem;

/// 应用后校验：cargo check -p <framework 依赖名>，使 Cargo.lock 与新配置保持一致
pub fn verify(items: &[RustPlanItem]) -> Result<()> {
    let mut groups: Vec<(PathBuf, Vec<String>)> = vec![];
    for item in items {
        let directory = item
            .file
            .parent()
            .with_context(|| format!("{} 没有上级目录", item.file.display()))?;
        let index = group(&mut groups, workspace_root(directory)?);

        let names = &mut groups[index].1;
        for dep in &item.deps {
            if !names.contains(&dep.name) {
                names.push(dep.name.clone());
            }
        }
    }

    for (root, names) in &groups {
        let packages = names
            .iter()
            .map(|name| format!("-p {name}"))
            .collect::<Vec<_>>()
            .join(" ");
        let line = format!("cargo check {packages}");
        println!("[Rust 校验] {line}  ({})", root.display());
        command::shell_inherit(root, &line)?;
    }

    Ok(())
}

/// 按 workspace root 分组，返回分组下标
fn group(groups: &mut Vec<(PathBuf, Vec<String>)>, root: PathBuf) -> usize {
    if let Some(index) = groups.iter().position(|(path, _)| path == &root) {
        return index;
    }

    groups.push((root, vec![]));
    groups.len() - 1
}

/// 通过 cargo metadata 获取 workspace root
fn workspace_root(directory: &Path) -> Result<PathBuf> {
    let output = command::shell_capture(directory, "cargo metadata --format-version 1")?;
    let metadata: Value = serde_json::from_str(&output).context("cargo metadata 输出解析失败")?;
    let root = metadata
        .get("workspace_root")
        .and_then(Value::as_str)
        .with_context(|| {
            format!(
                "cargo metadata 未返回 workspace_root: {}",
                directory.display()
            )
        })?;

    Ok(PathBuf::from(root))
}
