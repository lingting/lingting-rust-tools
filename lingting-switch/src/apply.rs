use anyhow::Result;

use crate::config::LocalDirectorys;
use crate::front;
use crate::plan::Plan;
use crate::rust;

/// 按计划执行修改
pub fn run(plan: &Plan, directorys: &LocalDirectorys) -> Result<()> {
    for item in &plan.rust {
        println!("[Apply] {} → {}", item.file.display(), item.target);
        rust::cargo_file::apply(&item.file, item.target, &item.deps)?;
    }

    // 改写后重新检查，使 Cargo.lock 与新配置保持一致
    rust::verify_applied(&plan.rust)?;

    for item in &plan.front {
        println!("[Apply] {} → {}", item.directory.display(), item.target);
        front::apply(item, directorys)?;
    }

    Ok(())
}
