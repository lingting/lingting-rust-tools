use std::path::Path;

use crate::plan::FrontPlanItem;

/// vp fmt 可以格式化的文件扩展名
const EXTENSIONS: [&str; 3] = ["ts", "tsx", "json"];

/// 本次修改涉及的文件名，仅保留格式化工具可处理的扩展名
pub fn files(item: &FrontPlanItem) -> String {
    item.files
        .iter()
        .filter(|file| is_formattable(file))
        .filter_map(|file| file.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_formattable(file: &Path) -> bool {
    file.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| EXTENSIONS.contains(&extension))
}
