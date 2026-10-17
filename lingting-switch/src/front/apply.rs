use anyhow::Result;

use crate::common::command;
use crate::config::LocalDirectorys;
use crate::junction;
use crate::mode::Mode;
use crate::plan::FrontPlanItem;

use super::{LRI_DIRECTORY, LRI_PACKAGE, format, package_json, pnpm_workspace, tsconfig, vite};

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
        // 切换为依赖模式：改写配置后删除 Junction
        Mode::Dependency => {
            let version = package.version()?;
            package_json::set_dependency(&directory.join("package.json"), LRI_PACKAGE, &version)?;
            config.switch_to_dependency()?;
            tsconfig::switch(&tsconfig_path, item.target)?;
            pnpm_workspace::ensure_excluded(&directory.join("pnpm-workspace.yaml"), LRI_PACKAGE)?;
            junction::remove_link(&link, &directorys.lri)?;
        }
    }

    after_apply(item)
}

/// 应用后操作：安装依赖，并只格式化本次修改的文件
fn after_apply(item: &FrontPlanItem) -> Result<()> {
    let directory = &item.directory;
    println!("[前端安装] pnpm install  ({})", directory.display());
    command::shell_inherit(directory, "pnpm install")?;

    let files = format::files(item);
    if !files.is_empty() {
        let line = format!("pnpm exec vp fmt {files}");
        println!("[前端格式化] {line}  ({})", directory.display());
        command::shell_inherit(directory, &line)?;
    }

    Ok(())
}
