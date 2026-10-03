use std::env;
use std::path::PathBuf;
use std::sync::LazyLock;

#[derive(Clone)]
pub struct Directorys {
    pub startup: PathBuf,
    pub install: PathBuf,
}

static DIRECTORYS: LazyLock<Directorys> = LazyLock::new(|| Directorys {
    startup: env::current_dir().expect("未获取到运行目录"),
    install: env::current_exe().expect("未获取到安装目录"),
});

pub fn directorys() -> Directorys {
    DIRECTORYS.clone()
}
