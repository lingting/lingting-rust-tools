use std::cmp::Reverse;

use anyhow::{Result, bail};

/// 一次按字节范围替换文本的修改
#[derive(Debug, Clone)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

impl Edit {
    pub fn new(start: usize, end: usize, text: String) -> Self {
        Self { start, end, text }
    }
}

/// 按字节范围应用全部修改，范围重叠时返回错误
pub fn apply(source: &str, edits: Vec<Edit>) -> Result<String> {
    let mut sorted = edits;
    sorted.sort_by_key(|edit| Reverse(edit.start));

    let mut result = source.to_string();
    let mut boundary = source.len();
    for edit in sorted {
        if edit.end > boundary {
            bail!("修改范围重叠: {}..{}", edit.start, edit.end);
        }

        result.replace_range(edit.start..edit.end, &edit.text);
        boundary = edit.start;
    }

    Ok(result)
}

/// 在容器闭合符号前插入一项，不带缩进，格式交由项目格式化工具统一
pub fn append(end: usize, separator: &str, item: &str) -> Edit {
    let position = end.saturating_sub(1);
    Edit::new(position, position, format!("{separator}\n{item}"))
}

/// 字符串数组文本
pub fn array_text(items: &[String]) -> String {
    let body = items
        .iter()
        .map(|item| format!("\"{item}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!("[{body}]")
}
