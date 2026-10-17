use std::path::Path;

use anyhow::{Context, Result, bail};
use yaml_rust2::{Yaml, YamlLoader};

use crate::common::text_file;

/// pnpm 的发布年龄白名单键名
const KEY: &str = "minimumReleaseAgeExclude";
/// 新增序列条目使用的缩进
const INDENT: &str = "  ";

/// 判断白名单是否已包含该依赖
pub fn excluded(path: &Path, name: &str) -> Result<bool> {
    Ok(matches!(&load(path)?[KEY], Yaml::Array(items) if contains(items, name)))
}

/// 确保白名单包含该依赖，返回是否发生修改
pub fn ensure_excluded(path: &Path, name: &str) -> Result<bool> {
    match &load(path)?[KEY] {
        Yaml::Array(items) if contains(items, name) => return Ok(false),
        // 键不存在时为 BadValue，与空值一样按新增处理
        Yaml::Array(_) | Yaml::Null | Yaml::BadValue => {}
        other => bail!("{KEY} 不是序列: {other:?}"),
    }

    let source = text_file::read(path)?;
    let updated = append(&source, name);
    text_file::write(path, &updated)?;
    Ok(true)
}

fn contains(items: &[Yaml], name: &str) -> bool {
    items.iter().any(|item| item.as_str() == Some(name))
}

/// 在 minimumReleaseAgeExclude 序列末尾追加一个条目
fn append(source: &str, name: &str) -> String {
    let ending = text_file::line_ending(source);
    let key = format!("{KEY}:");
    let lines = source
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect::<Vec<_>>();

    let Some(index) = lines.iter().position(|line| *line == key) else {
        let mut text = source.to_string();
        if !text.is_empty() && !text.ends_with('\n') {
            text.push_str(ending);
        }

        text.push_str(&format!("{key}{ending}{INDENT}- {name}{ending}"));
        return text;
    };

    // 序列块在第一个空行或第一个非缩进行处结束
    let mut end = index + 1;
    let mut indent = INDENT.to_string();
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() || !line.starts_with([' ', '\t']) {
            break;
        }

        indent = line[..line.len() - line.trim_start().len()].to_string();
        end += 1;
    }

    let mut text = lines[..end].join(ending);
    text.push_str(ending);
    text.push_str(&format!("{indent}- {name}"));
    if end < lines.len() {
        text.push_str(ending);
        text.push_str(&lines[end..].join(ending));
    }

    text
}

fn load(path: &Path) -> Result<Yaml> {
    let source = text_file::read(path)?;
    let mut documents = YamlLoader::load_from_str(&source)
        .with_context(|| format!("YAML 解析失败: {}", path.display()))?;
    if documents.is_empty() {
        bail!("{} 内容为空", path.display());
    }

    Ok(documents.remove(0))
}
