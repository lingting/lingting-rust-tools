pub mod package_json;
pub mod pnpm_workspace;
pub mod tsconfig;
pub mod vite;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::command;
use crate::config::LocalDirectorys;
use crate::junction;
use crate::mode::Mode;
use crate::plan::FrontPlanItem;

/// lri 的包名
pub const LRI_PACKAGE: &str = "lingting-react-ui";
/// @lri 指向的本地目录名
pub const LRI_DIRECTORY: &str = "lri";
/// alias 中 lri 的键名
pub const LRI_ALIAS: &str = "@lri";

/// 前端项目必须同时存在的文件
const REQUIRED: [&str; 4] = [
    "package.json",
    "vite.config.ts",
    "pnpm-workspace.yaml",
    "tsconfig.json",
];

/// 收集运行目录及一级子目录下的前端项目
pub fn discover(startup: &Path) -> Result<Vec<PathBuf>> {
    let mut projects = vec![];
    push_project(&mut projects, startup);

    for entry in read_dir(startup)? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !entry.path().is_dir() {
            continue;
        }

        push_project(&mut projects, &entry.path());
    }

    Ok(projects)
}

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

/// 执行前端项目的切换
pub fn apply(item: &FrontPlanItem, directorys: &LocalDirectorys) -> Result<()> {
    let directory = &item.directory;
    let config = vite::ViteConfig::read(&directory.join("vite.config.ts"))?;
    let package = package_json::PackageJson::read(&directorys.lri_package_json())?;
    let tsconfig_path = directory.join("tsconfig.json");
    let link = directory.join(LRI_DIRECTORY);

    match item.target {
        // 切换为本地模式：创建 Junction，再改写 tsconfig 与 vite 配置
        Mode::Local => {
            junction::ensure(&link, &directorys.lri)?;
            tsconfig::switch(&tsconfig_path, item.target)?;
            config.switch_to_local(&package.peer_dependencies())?;
        }
        // 切换为依赖模式：改写配置后删除 Junction 并安装依赖
        Mode::Dependency => {
            let version = package.version()?;
            package_json::set_dependency(&directory.join("package.json"), LRI_PACKAGE, &version)?;
            config.switch_to_dependency()?;
            tsconfig::switch(&tsconfig_path, item.target)?;
            pnpm_workspace::ensure_excluded(&directory.join("pnpm-workspace.yaml"), LRI_PACKAGE)?;
            junction::remove_link(&link, &directorys.lri)?;
            command::shell_inherit(directory, "pnpm install")?;
        }
    }

    verify_applied(item)
}

/// 前端前置校验：改写前执行，只能使用只读命令
pub fn verify(item: &FrontPlanItem) -> Result<()> {
    run_check(&item.directory, false)
}

/// 前端应用后校验：改写后执行，使用项目自身命令，允许其顺带格式化
pub fn verify_applied(item: &FrontPlanItem) -> Result<()> {
    run_check(&item.directory, true)
}

fn run_check(directory: &Path, after_apply: bool) -> Result<()> {
    let line = command_line(directory, after_apply)?;
    let stage = if after_apply { "应用后" } else { "前置" };
    println!("[前端{stage}校验] {line}  ({})", directory.display());

    command::shell_inherit(directory, &line)
}

/// 校验命令：存在 lint script 时优先使用；改写前遇到会写回文件的命令时退回只读等价命令
fn command_line(directory: &Path, after_apply: bool) -> Result<String> {
    let package = package_json::PackageJson::read(&directory.join("package.json"))?;
    match package.script("lint") {
        Some(script) if after_apply || !writes_files(script) => Ok("pnpm lint".to_string()),
        _ => Ok(vite_plus_command(!after_apply)),
    }
}

/// vite-plus 的格式、lint 与类型检查命令链
fn vite_plus_command(read_only: bool) -> String {
    let format = if read_only {
        "vp fmt --check"
    } else {
        "vp fmt"
    };

    format!("pnpm exec {format} && pnpm exec vp lint && pnpm exec tsc --noEmit")
}

/// 判断命令组合是否会写回文件
fn writes_files(script: &str) -> bool {
    script.split("&&").any(|command| {
        let command = command.trim();
        command.contains("--fix")
            || command.contains("--write")
            || (is_formatter(command) && !command.contains("--check"))
    })
}

fn is_formatter(command: &str) -> bool {
    ["vp fmt", "vp format", "vp check", "oxfmt", "prettier"]
        .iter()
        .any(|name| command.starts_with(name))
}

fn push_project(projects: &mut Vec<PathBuf>, directory: &Path) {
    if REQUIRED.iter().all(|name| directory.join(name).is_file()) {
        projects.push(directory.to_path_buf());
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
