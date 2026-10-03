use anyhow::{Context, Result};
use clap::Parser;
use lingting_tools_core::directory::directorys;

use crate::apply;
use crate::cli::Cli;
use crate::config;
use crate::front;
use crate::git;
use crate::plan::Plan;
use crate::rust;

/// 执行完整流程：Plan → Git 检查 → Apply
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let local = config::local_directorys()?;
    let startup = directorys().startup;
    let root = git::repository_root(&startup)?;
    println!("[Git] 仓库根目录: {}", root.display());

    let target = cli.mode.map(Into::into);
    let plan = Plan {
        rust: rust::plan(&startup, &local, target)?,
        front: front::plan(&startup, &local, target)?,
    };

    if plan.is_empty() {
        println!("未发现需要切换的配置");
        return Ok(());
    }

    if cli.dry_run {
        plan.print(&startup);
        return Ok(());
    }

    git::ensure_ready(&root, &plan.files())?;

    plan.print(&startup);
    if !cli.yes && !confirm()? {
        println!("已取消");
        return Ok(());
    }

    apply::run(&plan, &local)
}

/// 读取确认输入，回车或 y 表示继续
fn confirm() -> Result<bool> {
    println!("[y/Enter] Apply");
    println!("[其他输入] Cancel");

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("读取输入失败")?;
    let answer = input.trim();

    Ok(answer.is_empty() || answer.eq_ignore_ascii_case("y"))
}
