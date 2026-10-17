use std::path::{Path, PathBuf};

use crate::common::path;
use crate::mode::Mode;

/// Cargo.toml 中一个待切换的 framework 依赖
#[derive(Debug, Clone)]
pub struct RustDepPlan {
    /// 依赖所属的表，例如 ["workspace", "dependencies"]
    pub table: Vec<String>,
    /// 依赖在 Cargo.toml 中的键名
    pub key: String,
    /// framework 包名
    pub name: String,
    /// 当前模式
    pub current: Mode,
    /// 目标模式要写入的值：本地模式为包目录，依赖模式为版本号
    pub value: String,
}

/// 一个 Cargo.toml 的切换计划
#[derive(Debug, Clone)]
pub struct RustPlanItem {
    pub file: PathBuf,
    pub target: Mode,
    pub deps: Vec<RustDepPlan>,
}

/// 一个前端项目的切换计划
#[derive(Debug, Clone)]
pub struct FrontPlanItem {
    pub directory: PathBuf,
    pub current: Mode,
    pub target: Mode,
    /// 实际需要修改的文件
    pub files: Vec<PathBuf>,
}

/// 完整的切换计划
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub rust: Vec<RustPlanItem>,
    pub front: Vec<FrontPlanItem>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.rust.is_empty() && self.front.is_empty()
    }

    /// 全部需要修改的文件
    pub fn files(&self) -> Vec<PathBuf> {
        let mut files = self
            .rust
            .iter()
            .map(|item| item.file.clone())
            .collect::<Vec<_>>();

        for item in &self.front {
            files.extend(item.files.iter().cloned());
        }

        files
    }

    /// 输出计划内容
    pub fn print(&self, startup: &Path) {
        for item in &self.rust {
            println!("{}", path::to_slash(display_path(&item.file, startup)));
            for dep in &item.deps {
                println!("  {}: {} → {}", dep.name, dep.current, item.target);
            }
        }

        for item in &self.front {
            let directory = path::to_slash(display_path(&item.directory, startup));
            println!("{directory}/");
            println!("  lri: {} → {}", item.current, item.target);
        }
    }
}

/// 相对运行目录展示路径，位于运行目录之外时保留绝对路径
fn display_path<'a>(path: &'a Path, startup: &Path) -> &'a Path {
    path.strip_prefix(startup).unwrap_or(path)
}
