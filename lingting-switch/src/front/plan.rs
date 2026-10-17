use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::common::dir;
use crate::config::LocalDirectorys;
use crate::mode::Mode;
use crate::plan::FrontPlanItem;

use super::{LRI_PACKAGE, package_json, pnpm_workspace, tsconfig, vite};

/// 前端项目必须同时存在的文件
const REQUIRED: [&str; 4] = [
    "package.json",
    "vite.config.ts",
    "pnpm-workspace.yaml",
    "tsconfig.json",
];

/// 扫描前端项目并生成计划
pub fn plan(
    startup: &Path,
    directorys: &LocalDirectorys,
    target: Option<Mode>,
) -> Result<Vec<FrontPlanItem>> {
    let mut items = vec![];
    for directory in discover(startup)? {
        let config = vite::ViteConfig::read(&directory.join("vite.config.ts"))?;
        let current = config.mode()?;
        let target = target.unwrap_or_else(|| current.reverse());
        if target == current {
            continue;
        }

        let package = package_json::PackageJson::read(&directorys.lri_package_json())?;
        let tsconfig = directory.join("tsconfig.json");
        tsconfig::edits(&tsconfig, target)?;

        let mut files = vec![directory.join("vite.config.ts"), tsconfig];
        match target {
            Mode::Local => {
                config.local_edits(&package.peer_dependencies())?;
            }
            Mode::Dependency => {
                package.version()?;
                config.dependency_edits()?;

                files.push(directory.join("package.json"));
                let workspace = directory.join("pnpm-workspace.yaml");
                if !pnpm_workspace::excluded(&workspace, LRI_PACKAGE)? {
                    files.push(workspace);
                }
            }
        }

        items.push(FrontPlanItem {
            directory,
            current,
            target,
            files,
        });
    }

    Ok(items)
}

/// 运行目录及一级子目录下的前端项目
fn discover(startup: &Path) -> Result<Vec<PathBuf>> {
    dir::discover(startup, is_project)
}

/// 必需文件齐全时才算前端项目
fn is_project(directory: &Path) -> bool {
    REQUIRED.iter().all(|name| directory.join(name).is_file())
}
