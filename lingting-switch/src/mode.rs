use std::fmt::{Display, Formatter};

/// 配置模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 本地模式：framework / lri 指向本地目录
    Local,
    /// 依赖模式：framework / lri 指向已发布的包
    Dependency,
}

impl Mode {
    /// 取相反模式
    pub fn reverse(self) -> Self {
        match self {
            Mode::Local => Mode::Dependency,
            Mode::Dependency => Mode::Local,
        }
    }
}

impl Display for Mode {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Local => write!(formatter, "local"),
            Mode::Dependency => write!(formatter, "dependency"),
        }
    }
}
