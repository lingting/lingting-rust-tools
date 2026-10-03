use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// 执行命令并返回标准输出
pub fn capture(directory: &Path, command: &str) -> Result<String> {
    let output = Command::new("cmd")
        .arg("/C")
        .arg(command)
        .current_dir(directory)
        .output()
        .with_context(|| format!("无法执行命令: {command}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!("命令执行失败: {command}\n{stderr}{stdout}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 执行命令并把输出直接交给终端，用于耗时较长的校验命令
pub fn inherit(directory: &Path, command: &str) -> Result<()> {
    let status = Command::new("cmd")
        .arg("/C")
        .arg(command)
        .current_dir(directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("无法执行命令: {command}"))?;

    if !status.success() {
        bail!("命令执行失败: {command}");
    }

    Ok(())
}
