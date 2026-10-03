use clap::{Parser, ValueEnum};

use crate::mode::Mode;

/// 命令行指定的目标模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TargetMode {
    /// 切换到本地 framework / lri
    Local,
    /// 切换到已发布的 framework / lri 包
    Dependency,
}

impl From<TargetMode> for Mode {
    fn from(value: TargetMode) -> Self {
        match value {
            TargetMode::Local => Mode::Local,
            TargetMode::Dependency => Mode::Dependency,
        }
    }
}

/// 快速切换项目的 framework 与 lri 配置
#[derive(Debug, Parser)]
#[command(name = "lingting-switch", version, about)]
pub struct Cli {
    /// 只生成 Plan，不做校验也不修改文件
    #[arg(short = 'd', long = "dry-run", conflicts_with = "yes")]
    pub dry_run: bool,

    /// Plan 与校验通过后直接执行，不再询问
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// 强制指定目标模式，省略时按当前模式自动反向切换
    #[arg(value_name = "MODE", value_enum)]
    pub mode: Option<TargetMode>,
}
