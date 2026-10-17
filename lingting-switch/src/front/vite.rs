use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tree_sitter::{Node, Parser, Tree};
use tree_sitter_typescript::LANGUAGE_TYPESCRIPT;

use crate::common::source_edit::{self, Edit};
use crate::common::text_file;
use crate::mode::Mode;

use super::{LRI_ALIAS, LRI_DIRECTORY, LRI_PACKAGE};

/// @lri 在指定模式下对应的写法
pub fn alias_value(mode: Mode) -> String {
    match mode {
        Mode::Local => format!("resolve(import.meta.dirname, \"{LRI_DIRECTORY}\")"),
        Mode::Dependency => format!("\"{LRI_PACKAGE}\""),
    }
}

/// vite.config.ts 及其语法树
pub struct ViteConfig {
    pub path: PathBuf,
    source: String,
    tree: Tree,
}

impl ViteConfig {
    /// 读取并解析 vite.config.ts
    pub fn read(path: &Path) -> Result<Self> {
        let source = text_file::read(path)?;
        let mut parser = Parser::new();
        parser
            .set_language(&LANGUAGE_TYPESCRIPT.into())
            .with_context(|| format!("加载 TypeScript 语法失败: {}", path.display()))?;
        let tree = parser
            .parse(&source, None)
            .with_context(|| format!("解析失败: {}", path.display()))?;

        Ok(Self {
            path: path.to_path_buf(),
            source,
            tree,
        })
    }

    /// 识别 @lri 的当前模式
    pub fn mode(&self) -> Result<Mode> {
        let node = self.alias_value_node().with_context(|| {
            format!(
                "{} 的 resolve.alias 中未找到 {LRI_ALIAS}",
                self.path.display()
            )
        })?;
        let text = self.text(node);

        if text == alias_value(Mode::Dependency) {
            return Ok(Mode::Dependency);
        }

        if normalize(text) == normalize(&alias_value(Mode::Local)) {
            return Ok(Mode::Local);
        }

        bail!("无法识别 {LRI_ALIAS} 的当前模式: {text}")
    }

    /// 本地模式的全部修改
    pub fn local_edits(&self, peers: &[String]) -> Result<Vec<Edit>> {
        Ok(vec![
            self.alias_edit(Mode::Local)?,
            self.dedupe_edit(peers)?,
            self.ignore_patterns_edit()?,
        ])
    }

    /// 依赖模式的全部修改
    pub fn dependency_edits(&self) -> Result<Vec<Edit>> {
        Ok(vec![self.alias_edit(Mode::Dependency)?])
    }

    /// 写入本地模式
    pub fn switch_to_local(&self, peers: &[String]) -> Result<()> {
        self.write(self.local_edits(peers)?)
    }

    /// 写入依赖模式
    pub fn switch_to_dependency(&self) -> Result<()> {
        self.write(self.dependency_edits()?)
    }

    fn write(&self, edits: Vec<Edit>) -> Result<()> {
        let updated = source_edit::apply(&self.source, edits)?;
        text_file::write(&self.path, &updated)
    }

    /// 覆盖 @lri 的写法
    fn alias_edit(&self, mode: Mode) -> Result<Edit> {
        let node = self
            .alias_value_node()
            .with_context(|| format!("resolve.alias 中未找到 {LRI_ALIAS}"))?;
        Ok(Edit::new(
            node.start_byte(),
            node.end_byte(),
            alias_value(mode),
        ))
    }

    /// dedupe 追加 lri 的 peerDependencies，缺少 dedupe 时补写
    fn dedupe_edit(&self, peers: &[String]) -> Result<Edit> {
        let resolve = self.resolve_object()?;
        let Some(pair) = find_property(resolve, &self.source, "dedupe") else {
            let item = format!("dedupe: {}", source_edit::array_text(peers));
            return Ok(source_edit::append(
                resolve.end_byte(),
                separator(resolve),
                &item,
            ));
        };

        let array = self.array_node(pair, "dedupe")?;
        let mut items = self.array_strings(array)?;
        for peer in peers {
            if !items.contains(peer) {
                items.push(peer.clone());
            }
        }

        Ok(self.array_edit(array, &items))
    }

    /// ignorePatterns 追加本地 lri 目录，缺少 ignorePatterns 时补写
    fn ignore_patterns_edit(&self) -> Result<Edit> {
        let Some(pair) = find_property(self.tree.root_node(), &self.source, "ignorePatterns")
        else {
            return self.insert_ignore_patterns();
        };

        let array = self.array_node(pair, "ignorePatterns")?;
        let mut items = self.array_strings(array)?;
        if !items.iter().any(|item| item == LRI_DIRECTORY) {
            items.push(LRI_DIRECTORY.to_string());
        }

        Ok(self.array_edit(array, &items))
    }

    /// ignorePatterns 缺失时补进 fmt，fmt 也缺失时补进配置对象
    fn insert_ignore_patterns(&self) -> Result<Edit> {
        let resolve = self
            .property("resolve")
            .with_context(|| format!("{} 中未找到 resolve", self.path.display()))?;
        let config = resolve.parent().context("resolve 不在对象中")?;
        if config.kind() != "object" {
            bail!("resolve 不在对象中");
        }

        let patterns = source_edit::array_text(&patterns());
        let Some(pair) = find_property(config, &self.source, "fmt") else {
            let item = format!("fmt: {{ ignorePatterns: {patterns} }}");
            return Ok(source_edit::append(
                config.end_byte(),
                separator(config),
                &item,
            ));
        };

        let object = value(pair).context("fmt 不是对象")?;
        if object.kind() != "object" {
            bail!("fmt 不是对象");
        }

        let item = format!("ignorePatterns: {patterns}");
        Ok(source_edit::append(
            object.end_byte(),
            separator(object),
            &item,
        ))
    }

    fn array_edit(&self, array: Node<'_>, items: &[String]) -> Edit {
        Edit::new(
            array.start_byte(),
            array.end_byte(),
            source_edit::array_text(items),
        )
    }

    /// resolve 对象
    fn resolve_object(&self) -> Result<Node<'_>> {
        let resolve = self
            .property("resolve")
            .with_context(|| format!("{} 中未找到 resolve", self.path.display()))?;
        let object = value(resolve).context("resolve 不是对象")?;
        if object.kind() != "object" {
            bail!("resolve 不是对象");
        }

        Ok(object)
    }

    fn array_node<'s>(&'s self, pair: Node<'s>, name: &str) -> Result<Node<'s>> {
        let node = value(pair).with_context(|| format!("{name} 没有取值"))?;
        if node.kind() != "array" {
            bail!("{name} 不是数组");
        }

        Ok(node)
    }

    /// 数组中的字符串元素
    fn array_strings<'s>(&'s self, array: Node<'s>) -> Result<Vec<String>> {
        let mut items = vec![];
        let mut cursor = array.walk();
        for child in array.named_children(&mut cursor) {
            if child.kind() == "string" {
                items.push(unquote(self.text(child)).to_string());
            }
        }

        Ok(items)
    }

    /// @lri 的取值节点
    fn alias_value_node(&self) -> Option<Node<'_>> {
        let object = value(self.property("resolve")?)?;
        let object = value(find_property(object, &self.source, "alias")?)?;
        value(find_property(object, &self.source, LRI_ALIAS)?)
    }

    /// 属性所在的 pair 节点
    fn property(&self, name: &str) -> Option<Node<'_>> {
        find_property(self.tree.root_node(), &self.source, name)
    }

    fn text<'s>(&'s self, node: Node<'s>) -> &'s str {
        &self.source[node.start_byte()..node.end_byte()]
    }
}

/// 本地模式需要写入 ignorePatterns 的条目
fn patterns() -> Vec<String> {
    vec![LRI_DIRECTORY.to_string()]
}

/// 深度查找指定属性名的 pair 节点
fn find_property<'a>(node: Node<'a>, source: &str, name: &str) -> Option<Node<'a>> {
    if node.kind() == "pair" && property_name(node, source).as_deref() == Some(name) {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_property(child, source, name) {
            return Some(found);
        }
    }

    None
}

/// 新属性前是否需要逗号
fn separator(object: Node<'_>) -> &'static str {
    let mut cursor = object.walk();
    let children = object.children(&mut cursor).collect::<Vec<_>>();
    let Some(index) = children.iter().rposition(|child| child.kind() == "pair") else {
        return "";
    };

    if children[index + 1..]
        .iter()
        .any(|child| child.kind() == ",")
    {
        ""
    } else {
        ","
    }
}

/// pair 节点的键名，已去掉引号
fn property_name(pair: Node<'_>, source: &str) -> Option<String> {
    let key = pair.child_by_field_name("key")?;
    Some(unquote(text_of(key, source)).to_string())
}

/// pair 节点的取值
fn value<'a>(pair: Node<'a>) -> Option<Node<'a>> {
    pair.child_by_field_name("value")
}

fn text_of<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// 去掉字符串字面量的引号
fn unquote(text: &str) -> &str {
    let trim = text.trim();
    for quote in ['"', '\'', '`'] {
        if trim.len() >= 2 && trim.starts_with(quote) && trim.ends_with(quote) {
            return &trim[1..trim.len() - 1];
        }
    }

    trim
}

/// 去掉空白，用于比较写法差异
fn normalize(text: &str) -> String {
    text.chars().filter(|char| !char.is_whitespace()).collect()
}
