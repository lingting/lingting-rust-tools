//! 前端项目的 lri 切换

mod apply;
mod format;
pub mod package_json;
mod plan;
pub mod pnpm_workspace;
pub mod tsconfig;
pub mod vite;

pub use apply::apply;
pub use plan::plan;

/// lri 的包名
pub const LRI_PACKAGE: &str = "lingting-react-ui";
/// @lri 指向的本地目录名
pub const LRI_DIRECTORY: &str = "lri";
/// alias 中 lri 的键名
pub const LRI_ALIAS: &str = "@lri";
