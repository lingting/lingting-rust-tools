use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::command;

/// 确认运行目录位于 Git 仓库内，并返回仓库根目录
pub fn repository_root(startup: &Path) -> Result<PathBuf> {
    let output = command::run(startup, "git", &["rev-parse", "--show-toplevel"])
        .with_context(|| format!("{} 不在 Git 仓库内", startup.display()))?;
    let root = output.trim();
    if root.is_empty() {
        bail!("未获取到 Git 仓库根目录");
    }

    Ok(PathBuf::from(root))
}

/// 全部目标文件必须已有提交记录，且当前没有未提交变更
pub fn ensure_ready(root: &Path, files: &[PathBuf]) -> Result<()> {
    for file in files {
        let name = relative_path(root, file)?
            .to_string_lossy()
            .replace('\\', "/");

        let status = command::run(root, "git", &["status", "--porcelain", "--", &name])?;
        if !status.trim().is_empty() {
            bail!("目标文件存在未提交变更，已停止: {name}");
        }

        let history = command::run(root, "git", &["log", "-1", "--format=%H", "--", &name])?;
        if history.trim().is_empty() {
            bail!("目标文件没有 Git 提交记录，已停止: {name}");
        }

        println!("[Git] 已提交且 clean: {name}");
    }

    Ok(())
}

/// 计算相对仓库根目录的路径，忽略盘符大小写差异
fn relative_path(root: &Path, file: &Path) -> Result<PathBuf> {
    let root_components = root.components().collect::<Vec<_>>();
    let file_components = file.components().collect::<Vec<_>>();
    let outside = || format!("{} 不在仓库 {} 内", file.display(), root.display());

    if file_components.len() <= root_components.len() {
        bail!("{}", outside());
    }

    for (root_component, file_component) in root_components.iter().zip(file_components.iter()) {
        let left = root_component.as_os_str().to_string_lossy();
        let right = file_component.as_os_str().to_string_lossy();
        if !left.eq_ignore_ascii_case(&right) {
            bail!("{}", outside());
        }
    }

    let mut relative = PathBuf::new();
    for component in &file_components[root_components.len()..] {
        relative.push(component.as_os_str());
    }

    Ok(relative)
}
