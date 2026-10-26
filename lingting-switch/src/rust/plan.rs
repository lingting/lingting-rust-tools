use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::common::{dir, path};
use crate::config::LocalDirectorys;
use crate::mode::Mode;
use crate::plan::{RustDepPlan, RustPlanItem};

use super::cargo_file::{self, Source};
use super::framework;

/// 扫描 Rust 项目并生成计划
pub fn plan(
    startup: &Path,
    directorys: &LocalDirectorys,
    target: Option<Mode>,
) -> Result<Vec<RustPlanItem>> {
    let mut scanned = vec![];
    let mut framework_mode = None;
    let mut gpui_mode = None;
    for file in cargo_tomls(startup)? {
        let deps = cargo_file::scan(&file)?;
        if deps.is_empty() {
            continue;
        }

        // 未指定目标模式时，同一来源的依赖必须处于同一模式，否则无法反向切换
        if target.is_none() {
            for dep in &deps {
                let slot = match dep.source {
                    Source::Framework => &mut framework_mode,
                    Source::Gpui => &mut gpui_mode,
                };

                if let Some(mode) = slot {
                    if *mode != dep.mode {
                        bail!(
                            "检测到混合模式: {} 的 {} 为 {}，与 {} 不一致；请用参数指定目标模式",
                            file.display(),
                            dep.key,
                            dep.mode,
                            mode
                        );
                    }
                } else {
                    *slot = Some(dep.mode);
                }
            }
        }

        scanned.push((file, deps));
    }

    // 目标模式以 framework 的模式为准，两类依赖统一反向；没有 framework 依赖时退回 gpui
    let target = match target {
        Some(target) => target,
        None => match framework_mode.or(gpui_mode) {
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

            let root = match dep.source {
                Source::Framework => &directorys.framework,
                Source::Gpui => &directorys.gpui,
            };
            let value = match target {
                Mode::Local => local_path(root, &dep.name, &file)?,
                Mode::Dependency => framework::version(root, &dep.name)?,
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

/// 运行目录及一级子目录下的 Cargo.toml
fn cargo_tomls(startup: &Path) -> Result<Vec<PathBuf>> {
    let directories = dir::discover(startup, |directory| directory.join("Cargo.toml").is_file())?;

    Ok(directories
        .into_iter()
        .map(|directory| directory.join("Cargo.toml"))
        .collect())
}

/// 本地模式下依赖的 path 值：相对 Cargo.toml 所在目录，无法相对化时退回绝对路径
fn local_path(root: &Path, name: &str, file: &Path) -> Result<String> {
    let directory = framework::ensure_package(root, name)?;
    let base = file
        .parent()
        .with_context(|| format!("{} 没有上级目录", file.display()))?;
    let value = path::relative(base, &directory).unwrap_or(directory);

    Ok(path::to_slash(&value))
}
