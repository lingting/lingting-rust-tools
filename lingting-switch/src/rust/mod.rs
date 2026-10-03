pub mod cargo_file;
pub mod framework;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::command;
use crate::config::LocalDirectorys;
use crate::mode::Mode;
use crate::plan::{RustDepPlan, RustPlanItem};

/// 收集运行目录及一级子目录下的 Cargo.toml
pub fn discover(startup: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    push_cargo_toml(&mut files, startup);

    for entry in read_dir(startup)? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !entry.path().is_dir() {
            continue;
        }

        push_cargo_toml(&mut files, &entry.path());
    }

    Ok(files)
}

/// 扫描 Rust 项目并生成计划
pub fn plan(
    startup: &Path,
    directorys: &LocalDirectorys,
    target: Option<Mode>,
) -> Result<Vec<RustPlanItem>> {
    let mut scanned = vec![];
    let mut detection = None;
    for file in discover(startup)? {
        let deps = cargo_file::scan(&file)?;
        if deps.is_empty() {
            continue;
        }

        // 未指定目标模式时，全部依赖必须处于同一模式，否则无法反向切换
        if target.is_none() {
            for dep in &deps {
                if let Some(mode) = detection {
                    if mode != dep.mode {
                        bail!(
                            "检测到混合模式: {} 的 {} 为 {}，与 {} 不一致；请用参数指定目标模式",
                            file.display(),
                            dep.key,
                            dep.mode,
                            mode
                        );
                    }
                } else {
                    detection = Some(dep.mode);
                }
            }
        }

        scanned.push((file, deps));
    }

    let target = match target {
        Some(target) => target,
        None => match detection {
            Some(mode) => mode.reverse(),
            None => return Ok(vec![]),
        },
    };

    let mut items = vec![];
    for (file, deps) in scanned {
        let mut plans = vec![];
        for dep in deps {
            if dep.mode == target {
                continue;
            }

            let value = match target {
                Mode::Local => local_path(directorys, &dep.name)?,
                Mode::Dependency => framework::version(&directorys.framework, &dep.name)?,
            };

            plans.push(RustDepPlan {
                table: dep.table,
                key: dep.key,
                name: dep.name,
                current: dep.mode,
                value,
            });
        }

        if plans.is_empty() {
            continue;
        }

        items.push(RustPlanItem {
            file,
            target,
            deps: plans,
        });
    }

    Ok(items)
}

/// Rust 侧前置校验：cargo check -p <framework 依赖名>
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

/// 本地模式下依赖的 path 值
fn local_path(directorys: &LocalDirectorys, name: &str) -> Result<String> {
    let directory = framework::ensure_package(&directorys.framework, name)?;
    Ok(directory.to_string_lossy().replace('\\', "/"))
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

fn push_cargo_toml(files: &mut Vec<PathBuf>, directory: &Path) {
    let file = directory.join("Cargo.toml");
    if file.is_file() {
        files.push(file);
    }
}

fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let mut entries = vec![];
    for entry in fs::read_dir(path).with_context(|| format!("无法读取目录: {}", path.display()))?
    {
        entries.push(entry.with_context(|| format!("无法读取目录项: {}", path.display()))?);
    }

    entries.sort_by_key(|entry| entry.file_name());
    Ok(entries)
}
