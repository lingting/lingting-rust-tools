use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Map, Value};

use crate::common::text_file;

/// 一份 package.json
#[derive(Debug, Clone)]
pub struct PackageJson {
    pub path: PathBuf,
    value: Value,
}

impl PackageJson {
    pub fn read(path: &Path) -> Result<Self> {
        let text = text_file::read(path)?;
        let value = serde_json::from_str(&text)
            .with_context(|| format!("package.json 解析失败: {}", path.display()))?;

        Ok(Self {
            path: path.to_path_buf(),
            value,
        })
    }

    /// 包版本
    pub fn version(&self) -> Result<String> {
        self.value
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string)
            .with_context(|| format!("{} 缺少 version", self.path.display()))
    }

    /// peerDependencies 声明的包名
    pub fn peer_dependencies(&self) -> Vec<String> {
        self.object("peerDependencies")
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn object(&self, key: &str) -> Option<&Map<String, Value>> {
        self.value.get(key).and_then(Value::as_object)
    }
}

/// dependencies 写入版本号，devDependencies 中的同名声明移除
pub fn set_dependency(path: &Path, name: &str, version: &str) -> Result<()> {
    let text = text_file::read(path)?;
    let mut value: Value = serde_json::from_str(&text)
        .with_context(|| format!("package.json 解析失败: {}", path.display()))?;

    let object = value
        .as_object_mut()
        .with_context(|| format!("{} 结构异常", path.display()))?;
    object
        .entry("dependencies")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .with_context(|| format!("dependencies 不是对象: {}", path.display()))?
        .insert(name.to_string(), Value::String(version.to_string()));

    if let Some(dev) = object
        .get_mut("devDependencies")
        .and_then(Value::as_object_mut)
    {
        dev.remove(name);
    }

    let mut output = serde_json::to_string_pretty(&value).context("package.json 序列化失败")?;
    if text.ends_with('\n') {
        output.push('\n');
    }

    text_file::write(path, &output)
}
