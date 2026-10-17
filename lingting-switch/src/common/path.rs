use std::path::{Path, PathBuf};

/// 统一为 / 分隔的路径文本，避免把 Windows 的反斜杠写入配置
pub fn to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// base 到 target 的相对路径，两者根前缀不同（例如 Windows 下跨盘符）时返回 None
pub fn relative(base: &Path, target: &Path) -> Option<PathBuf> {
    let base = base.canonicalize().ok()?;
    let target = target.canonicalize().ok()?;
    let base = base.components().collect::<Vec<_>>();
    let target = target.components().collect::<Vec<_>>();

    if base.first() != target.first() {
        return None;
    }

    let common = base
        .iter()
        .zip(target.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut path = PathBuf::new();
    for _ in common..base.len() {
        path.push("..");
    }
    for component in &target[common..] {
        path.push(component.as_os_str());
    }

    if path.as_os_str().is_empty() {
        path.push(".");
    }

    Some(path)
}
