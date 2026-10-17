use std::path::Path;

use anyhow::{Context, Result, anyhow};
use jsonc_parser::ast::{Object, ObjectProp, Value};
use jsonc_parser::common::Ranged;
use jsonc_parser::{CollectOptions, ParseOptions, parse_to_ast};

use crate::common::source_edit::{self, Edit};
use crate::common::text_file;
use crate::mode::Mode;

use super::{LRI_ALIAS, LRI_DIRECTORY, LRI_PACKAGE};

/// @lri 的通配路径键名
const ALIAS_WILDCARD: &str = "@lri/*";
/// include 必须包含的条目
const INCLUDES: [&str; 3] = ["src/**/*.ts", "src/**/*.tsx", "vite.config.ts"];

/// 计算 tsconfig.json 在目标模式下的修改
pub fn edits(path: &Path, mode: Mode) -> Result<Vec<Edit>> {
    let source = text_file::read(path)?;
    computed_edits(path, &source, mode)
}

/// 切换模式并写回
pub fn switch(path: &Path, mode: Mode) -> Result<()> {
    let source = text_file::read(path)?;
    let edits = computed_edits(path, &source, mode)?;
    text_file::write(path, &source_edit::apply(&source, edits)?)
}

fn computed_edits(path: &Path, source: &str, mode: Mode) -> Result<Vec<Edit>> {
    let root = parse(path, source)?;
    let root = root
        .as_object()
        .with_context(|| format!("{} 根节点不是对象", path.display()))?;
    let target = match mode {
        Mode::Local => format!("./{LRI_DIRECTORY}"),
        Mode::Dependency => format!("./node_modules/{LRI_PACKAGE}"),
    };

    let mut edits = paths_edits(source, root, &target)?;
    edits.push(include_edit(source, root)?);
    Ok(edits)
}

/// 覆盖或补写 compilerOptions.paths 下的 @lri 与 @lri/*
fn paths_edits(source: &str, root: &Object<'_>, target: &str) -> Result<Vec<Edit>> {
    let Some(compiler_options) = property(root, "compilerOptions") else {
        // compilerOptions 缺失：连同 paths 一起补写
        let paths = paths_text(target);
        let item = format!("\"compilerOptions\": {{ \"paths\": {paths} }}");
        return Ok(vec![source_edit::append(
            root.range.end,
            separator(source, root),
            &item,
        )]);
    };

    let options = compiler_options
        .value
        .as_object()
        .context("compilerOptions 不是对象")?;
    let Some(paths) = property(options, "paths") else {
        // paths 缺失：整块补写
        let item = format!("\"paths\": {}", paths_text(target));
        return Ok(vec![source_edit::append(
            options.range.end,
            separator(source, options),
            &item,
        )]);
    };

    let object = paths.value.as_object().context("paths 不是对象")?;
    let mut edits = vec![];
    let mut missing = vec![];
    for (key, suffix) in [(LRI_ALIAS, ""), (ALIAS_WILDCARD, "/*")] {
        let value = format!("[\"{target}{suffix}\"]");
        match property(object, key) {
            Some(prop) => {
                let range = prop.value.range();
                edits.push(Edit::new(range.start, range.end, value));
            }
            None => missing.push(format!("\"{key}\": {value}")),
        }
    }

    if !missing.is_empty() {
        edits.push(source_edit::append(
            object.range.end,
            separator(source, object),
            &missing.join(", "),
        ));
    }

    Ok(edits)
}

/// include 追加缺失的条目，缺少 include 时补写
fn include_edit(source: &str, root: &Object<'_>) -> Result<Edit> {
    let Some(prop) = property(root, "include") else {
        let item = format!(
            "\"include\": {}",
            source_edit::array_text(&include_items(&[]))
        );
        return Ok(source_edit::append(
            root.range.end,
            separator(source, root),
            &item,
        ));
    };

    let array = prop.value.as_array().context("include 不是数组")?;
    let items = include_items(
        &array
            .elements
            .iter()
            .filter_map(|element| element.as_string_lit().map(|lit| lit.value.to_string()))
            .collect::<Vec<_>>(),
    );

    Ok(Edit::new(
        array.range.start,
        array.range.end,
        source_edit::array_text(&items),
    ))
}

/// include 应包含的条目
fn include_items(existing: &[String]) -> Vec<String> {
    let mut items = existing.to_vec();
    for required in INCLUDES {
        if !items.iter().any(|item| item == required) {
            items.push(required.to_string());
        }
    }

    items
}

/// 生成 paths 对象文本
fn paths_text(target: &str) -> String {
    format!("{{ \"{LRI_ALIAS}\": [\"{target}\"], \"{ALIAS_WILDCARD}\": [\"{target}/*\"] }}")
}

/// 新属性前是否需要逗号
fn separator(source: &str, object: &Object<'_>) -> &'static str {
    let Some(last) = object.properties.last() else {
        return "";
    };

    let position = object.range.end.saturating_sub(1);
    if source[last.range.end..position].contains(',') {
        ""
    } else {
        ","
    }
}

fn property<'a>(object: &'a Object<'a>, name: &str) -> Option<&'a ObjectProp<'a>> {
    object
        .properties
        .iter()
        .find(|prop| prop.name.as_str() == name)
}

fn parse<'a>(path: &Path, source: &'a str) -> Result<Value<'a>> {
    let result = parse_to_ast(source, &CollectOptions::default(), &ParseOptions::default())
        .map_err(|error| anyhow!("{} 解析失败: {error:?}", path.display()))?;

    result
        .value
        .with_context(|| format!("{} 内容为空", path.display()))
}
